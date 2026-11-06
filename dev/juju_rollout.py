#!/usr/bin/env python3
"""Single-node Juju lab orchestration. Never reads or prints application credentials."""

import argparse
import grp
import hashlib
import json
import os
from pathlib import Path
import platform
import pwd
import re
import shutil
import subprocess
import sys
import tempfile
import time
import tomllib
import urllib.request

REGISTRY = "127.0.0.1:5000"
MARKER = "# Managed by app-center-ratings/dev/juju_rollout.py\n"


class LabError(RuntimeError):
    pass


def run(*command, capture=False, timeout=120):
    print("+ " + " ".join(map(str, command)), file=sys.stderr, flush=True)
    try:
        result = subprocess.run(
            list(map(str, command)),
            check=True,
            text=True,
            stdout=subprocess.PIPE if capture else None,
            stdin=subprocess.DEVNULL,
            timeout=timeout,
        )
    except subprocess.TimeoutExpired as exc:
        raise LabError(f"Command timed out after {timeout}s: {command[0]}") from exc
    return result.stdout if capture else None


def atomic_write(path, text, mode=0o600):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, name = tempfile.mkstemp(dir=path.parent, prefix=path.name + ".")
    try:
        os.fchmod(fd, mode)
        with os.fdopen(fd, "w") as stream:
            stream.write(text)
        os.replace(name, path)
    finally:
        Path(name).unlink(missing_ok=True)


def managed_write(path, text, mode=0o644):
    """Refuse to overwrite configuration not installed by these recipes."""
    path = Path(path)
    if path.exists():
        existing = path.read_text()
        if existing == text:
            return False
        if not existing.startswith(MARKER):
            raise LabError(f"Refusing to replace unmanaged configuration: {path}")
    atomic_write(path, text, mode)
    return True


def require_vm():
    release = platform.freedesktop_os_release()
    if release.get("ID") != "ubuntu" or release.get("VERSION_ID") != "24.04":
        raise LabError("Run this recipe only inside the Ubuntu 24.04 lab VM")
    if platform.machine() != "x86_64":
        raise LabError("The local rollout currently supports amd64 only")
    if os.geteuid() != 0:
        raise LabError("Registry preparation requires sudo inside the lab VM")
    nodes = json.loads(
        run("k8s", "kubectl", "get", "nodes", "-o", "json", capture=True)
    )
    if len(nodes["items"]) != 1:
        raise LabError(
            "A loopback registry only supports this single-node lab; use a shared registry for multiple nodes"
        )


def registry_setup():
    require_vm()
    config = tomllib.loads(Path("/etc/containerd/config.toml").read_text())
    registry = (
        config.get("plugins", {})
        .get("io.containerd.grpc.v1.cri", {})
        .get("registry", {})
    )
    if registry.get("config_path") != "/etc/containerd/hosts.d":
        raise LabError(
            "Expected Canonical Kubernetes containerd config_path=/etc/containerd/hosts.d; refusing to rewrite containerd"
        )
    hosts = Path("/etc/containerd/hosts.d") / REGISTRY / "hosts.toml"
    hosts_text = (
        MARKER
        + f"""server = "http://{REGISTRY}"
[host."http://{REGISTRY}"]
  capabilities = ["pull", "resolve", "push"]
"""
    )
    if (
        hosts.exists()
        and hosts.read_text() != hosts_text
        and not hosts.read_text().startswith(MARKER)
    ):
        raise LabError(f"Refusing to replace unmanaged registry settings: {hosts}")
    if not shutil.which("docker-registry"):
        # Avoid starting the package's default all-interface registry during install.
        run("systemctl", "mask", "docker-registry.service")
        run("apt-get", "update", timeout=300)
        run("apt-get", "install", "-y", "docker-registry", timeout=300)
    account = pwd.getpwnam("docker-registry")
    group = grp.getgrnam("docker-registry")
    root = Path("/etc/ratings-registry")
    root.mkdir(mode=0o750, exist_ok=True)
    os.chown(root, 0, group.gr_gid)
    secret_file = root / "http-secret"
    if not secret_file.exists():
        atomic_write(secret_file, os.urandom(32).hex() + "\n", 0o640)
        os.chown(secret_file, 0, group.gr_gid)
    secret = secret_file.read_text().strip()
    # JSON is also YAML; retain the marker for safe repeat runs.
    configuration = {
        "version": 0.1,
        "log": {"level": "warn"},
        "storage": {"filesystem": {"rootdirectory": "/var/lib/ratings-registry"}},
        "http": {"addr": REGISTRY, "secret": secret, "relativeurls": True},
    }
    changed = managed_write(
        root / "config.yaml", MARKER + json.dumps(configuration, indent=2) + "\n", 0o640
    )
    os.chown(root / "config.yaml", 0, group.gr_gid)
    state = Path("/var/lib/ratings-registry")
    state.mkdir(mode=0o700, exist_ok=True)
    os.chown(state, account.pw_uid, group.gr_gid)
    unit = (
        MARKER
        + """[Unit]
Description=Ratings single-node lab OCI registry (VM loopback only)
After=network.target

[Service]
User=docker-registry
Group=docker-registry
ExecStart=/usr/bin/docker-registry serve /etc/ratings-registry/config.yaml
Restart=on-failure
NoNewPrivileges=true
PrivateTmp=true
ProtectHome=true
ProtectSystem=strict
StateDirectory=ratings-registry
StateDirectoryMode=0700

[Install]
WantedBy=multi-user.target
"""
    )
    changed = (
        managed_write("/etc/systemd/system/ratings-registry.service", unit) or changed
    )
    managed_write(hosts, hosts_text)
    run("systemctl", "daemon-reload")
    run("systemctl", "enable", "--now", "ratings-registry.service")
    if changed:
        run("systemctl", "restart", "ratings-registry.service")
    for _ in range(20):
        try:
            with urllib.request.urlopen(
                f"http://{REGISTRY}/v2/", timeout=2
            ) as response:
                if response.status == 200:
                    print(
                        f"Registry ready at {REGISTRY}; no containerd restart required"
                    )
                    return
        except OSError:
            time.sleep(0.5)
    raise LabError(
        "Registry did not become ready; inspect journalctl -u ratings-registry"
    )


def publish(rock, state_dir):
    rock = Path(rock).resolve(strict=True)
    if rock.suffix != ".rock":
        raise LabError("Expected a packed .rock OCI archive")
    with rock.open("rb") as stream:
        checksum = hashlib.file_digest(stream, "sha256").hexdigest()
    tagged = f"{REGISTRY}/ratings:{checksum[:20]}"
    run(
        "skopeo",
        "copy",
        "--preserve-digests",
        "--dest-tls-verify=false",
        f"oci-archive:{rock}",
        f"docker://{tagged}",
        timeout=600,
    )
    digest = run(
        "skopeo",
        "inspect",
        "--tls-verify=false",
        "--format",
        "{{.Digest}}",
        f"docker://{tagged}",
        capture=True,
    ).strip()
    if not re.fullmatch(r"sha256:[0-9a-f]{64}", digest):
        raise LabError("Registry returned an invalid manifest digest")
    image = f"{REGISTRY}/ratings@{digest}"
    atomic_write(
        Path(state_dir) / "image.json",
        json.dumps({"image": image, "rock": str(rock), "sha256": checksum}) + "\n",
    )
    print(f"Published {image}")


def juju_json(*args):
    return json.loads(run("juju", *args, "--format=json", capture=True))


def related(status, application, endpoint, remote):
    relations = (
        status["applications"][application].get("relations", {}).get(endpoint, [])
    )
    return any(
        (item if isinstance(item, str) else item["related-application"]) == remote
        for item in relations
    )


def deploy(charm, state_dir, model):
    charm = Path(charm).resolve(strict=True)
    if charm.suffix != ".charm":
        raise LabError("Expected a packed .charm archive")
    image = json.loads((Path(state_dir) / "image.json").read_text())["image"]
    if not re.fullmatch(re.escape(REGISTRY) + r"/ratings@sha256:[0-9a-f]{64}", image):
        raise LabError("Invalid local image state; run ratings-image again")
    run(
        "skopeo",
        "inspect",
        "--tls-verify=false",
        "--format",
        "{{.Digest}}",
        f"docker://{image}",
        capture=True,
    )
    status = juju_json("status", "--model", model)
    apps = status["applications"]
    if (
        "postgresql" not in apps
        or apps["postgresql"].get("charm-name") != "postgresql-k8s"
    ):
        raise LabError(
            "Expected application postgresql (postgresql-k8s); run just vm postgres first"
        )
    if "ratings" in apps:
        if apps["ratings"].get("charm-name") != "ratings":
            raise LabError(
                "Refusing to refresh an unrelated application called ratings"
            )
        if apps["ratings"].get("relations", {}).get("database") and not related(
            status, "ratings", "database", "postgresql"
        ):
            raise LabError(
                "Ratings already has a different database relation; refusing to replace it"
            )
        run(
            "juju",
            "refresh",
            "ratings",
            "--model",
            model,
            "--path",
            charm,
            "--resource",
            f"ratings-image={image}",
            timeout=300,
        )
    else:
        run(
            "juju",
            "deploy",
            charm,
            "ratings",
            "--model",
            model,
            "--resource",
            f"ratings-image={image}",
            "--num-units=1",
            timeout=300,
        )
    status = juju_json("status", "--model", model)
    if not related(status, "ratings", "database", "postgresql"):
        if status["applications"]["ratings"].get("relations", {}).get("database"):
            raise LabError(
                "Ratings already has a different database relation; refusing to replace it"
            )
        run(
            "juju",
            "integrate",
            "--model",
            model,
            "ratings:database",
            "postgresql:database",
        )
    print("Ratings deployed/refreshed; database/JWT configuration is owned by Juju")


def observability(controller, model, cos_model):
    status = juju_json("status", "--model", model)
    if "ratings" not in status["applications"]:
        raise LabError("Deploy Ratings before integrating observability")
    cos = juju_json("status", "--model", cos_model)
    if cos["applications"].get("loki", {}).get("charm-name") != "loki-k8s":
        raise LabError(
            "Expected COS application loki (loki-k8s); run just vm cos first"
        )
    offer_name = "ratings-logs"
    offers = juju_json("offers", "--model", cos_model)
    if offer_name not in offers:
        source_model = cos_model.split(":", 1)[1]
        run(
            "juju",
            "offer",
            "--controller",
            controller,
            f"{source_model}.loki:logging",
            offer_name,
        )
        offers = juju_json("offers", "--model", cos_model)
    offer = offers[offer_name]
    if (
        offer.get("application") != "loki"
        or offer.get("endpoints", {}).get("logging", {}).get("interface")
        != "loki_push_api"
    ):
        raise LabError(
            "An unrelated ratings-logs offer already exists; refusing to reuse it"
        )
    offer_url = offer["offer-url"]
    alias = "ratings-loki"
    if alias not in status.get("application-endpoints", {}):
        run("juju", "consume", "--model", model, f"{controller}:{offer_url}", alias)
        status = juju_json("status", "--model", model)
    endpoint = status["application-endpoints"][alias]
    if (
        endpoint.get("url") != f"{controller}:{offer_url}"
        and endpoint.get("url") != offer_url
    ):
        raise LabError("Existing ratings-loki alias points to a different offer")
    # The Multipass lab's L2 pool is not a routable egress destination for pods.
    # Let Loki advertise its in-cluster endpoint instead of Traefik's LB address.
    if related(cos, "loki", "ingress", "traefik"):
        run(
            "juju",
            "remove-relation",
            "--model",
            cos_model,
            "loki:ingress",
            "traefik:ingress-per-unit",
        )
    if not related(status, "ratings", "logging", alias):
        run(
            "juju", "integrate", "--model", model, "ratings:logging", f"{alias}:logging"
        )
    print(
        "Ratings workload logs integrated with COS Loki; metrics/traces are not implemented"
    )


def health(status, application):
    app = status["applications"].get(application)
    if not app:
        return False, f"{application}: not present"
    units = app.get("units", {})
    summary = [
        f"{application}: {app.get('application-status', {}).get('current', 'unknown')}"
    ]
    ready = (
        bool(units)
        and len(units) == app.get("scale")
        and app.get("application-status", {}).get("current") == "active"
    )
    for name, unit in units.items():
        workload = unit.get("workload-status", {}).get("current", "unknown")
        agent = unit.get("juju-status", {}).get("current", "unknown")
        summary.append(f"{name}: workload={workload}, agent={agent}")
        ready = ready and workload == "active" and agent == "idle"
        if workload == "error" or agent == "error":
            raise LabError(
                f"{name} has a Juju hook error; inspect just vm ratings-status and juju debug-log before retrying"
            )
    return ready, "; ".join(summary)


def wait(model, timeout):
    deadline = time.monotonic() + timeout
    previous = None
    while time.monotonic() < deadline:
        status = juju_json("status", "--model", model)
        ready, summary = health(status, "ratings")
        # Provider application status may be stale; check each PostgreSQL unit too.
        database_ready, db_summary = health(status, "postgresql")
        summary += "; " + db_summary
        if summary != previous:
            print(summary, flush=True)
            previous = summary
        if ready and database_ready:
            print(
                "Ratings and PostgreSQL units are active/idle; host access: just ratings"
            )
            return
        time.sleep(5)
    raise LabError(
        f"Timed out after {timeout}s waiting for Ratings/PostgreSQL; inspect just vm ratings-status and ratings-logs"
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--controller", default="ratings-k8s")
    parser.add_argument("--model", default="ratings-k8s:ratings-dev")
    parser.add_argument("--cos-model", default="ratings-k8s:cos-lite")
    parser.add_argument(
        "--state-dir", type=Path, default=Path.home() / "ratings-lab-artifacts"
    )
    subparsers = parser.add_subparsers(dest="command", required=True)
    subparsers.add_parser("registry")
    publish_parser = subparsers.add_parser("publish")
    publish_parser.add_argument("rock", type=Path)
    deploy_parser = subparsers.add_parser("deploy")
    deploy_parser.add_argument("charm", type=Path)
    subparsers.add_parser("observability")
    wait_parser = subparsers.add_parser("wait")
    wait_parser.add_argument("--timeout", type=int, default=600)
    args = parser.parse_args()
    try:
        if args.command == "registry":
            registry_setup()
        elif args.command == "publish":
            publish(args.rock, args.state_dir)
        elif args.command == "deploy":
            deploy(args.charm, args.state_dir, args.model)
        elif args.command == "observability":
            observability(args.controller, args.model, args.cos_model)
        elif args.command == "wait":
            if args.timeout <= 0:
                raise LabError("Timeout must be positive")
            wait(args.model, args.timeout)
    except (
        LabError,
        subprocess.CalledProcessError,
        OSError,
        ValueError,
        KeyError,
    ) as exc:
        print(f"Rollout failed: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
