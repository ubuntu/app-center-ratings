#!/usr/bin/env python3
"""Opt-in, local-only HAProxy -> Traefik -> Ratings lab. No production offers."""

import argparse
import ipaddress
import json
import re
import ssl
import subprocess
import sys
import time

from juju_rollout import LabError, RATINGS_CHARMS, health, juju_json, run

CONTROLLER = "ratings-lxd"
MODEL = "ratings-k8s:ratings-dev"
EDGE_MODEL = f"{CONTROLLER}:ratings-edge"
HOSTNAME = "ratings.localhost"
# Pin the PS7 comparison, not whatever today's stable channels resolve to.
K8S_APPS = {
    "traefik": ("traefik-k8s", 377, "latest/stable", "20.04", True),
    "ingress-configurator": ("ingress-configurator", 95, "latest/stable", "24.04", True),
    "self-signed-certificates": ("self-signed-certificates", 586, "1/stable", "24.04", False),
}
EDGE_APPS = {
    "edge": ("haproxy", 551, "2.8/stable", "24.04", False),
    "edge-certificates": ("self-signed-certificates", 586, "1/stable", "24.04", False),
}
CONFIG = {
    "ratings": {"external-hostname": HOSTNAME},
    "traefik": {"external_hostname": HOSTNAME, "routing_mode": "path"},
    "ingress-configurator": {
        "hostname": HOSTNAME,
        "backend-protocol": "https",
        "paths": "/",
        "allow-http": False,
        "path-rewrite-expressions": "",
        "header-rewrite-expressions": "",
        "backend-addresses": "",
        "backend-ports": "",
    },
    "edge": {"external-hostname": HOSTNAME},
}
# Requirer first: single-provider endpoints must not silently acquire a second relation.
K8S_RELATIONS = (
    ("traefik:upstream-ingress", "ingress-configurator:ingress"),
    ("ratings:traefik-route", "traefik:traefik-route"),
    ("traefik:certificates", "self-signed-certificates:certificates"),
    ("ingress-configurator:haproxy-route", "edge:haproxy-route"),
)
EDGE_RELATIONS = (
    ("edge:certificates", "edge-certificates:certificates"),
    ("edge:receive-ca-certs", "traefik-ca:send-ca-cert"),
)


def check_model(model, kind, cloud):
    data = juju_json("show-model", "--model", model)
    if len(data) != 1:
        raise LabError(f"Expected exactly one model description for {model}")
    info = next(iter(data.values()))
    if info.get("model-type") != kind or info.get("cloud") != cloud:
        raise LabError(f"Refusing unrelated model {model}; expected {kind} on {cloud}")


def bootstrap():
    cloud = juju_json("show-cloud", "--client", "localhost")["localhost"]
    if cloud.get("type") != "lxd":
        raise LabError("Expected the VM's localhost LXD cloud from Concierge")
    controllers = juju_json("controllers")["controllers"]
    if CONTROLLER in controllers:
        if controllers[CONTROLLER].get("cloud") != "localhost":
            raise LabError(f"Refusing unrelated controller {CONTROLLER}")
        print(f"Reusing {CONTROLLER}; no bootstrap or refresh")
        return
    run("lxc", "info", "local:", capture=True, timeout=30)
    run(
        "juju", "bootstrap", "localhost", CONTROLLER, "--no-switch",
        "--bootstrap-base=ubuntu@24.04", "--config", "bootstrap-timeout=900",
        "--model-default", "test-mode=true", "--model-default", "automatically-retry-hooks=false",
        timeout=960,
    )


def edge_model():
    controllers = juju_json("controllers")["controllers"]
    if controllers.get(CONTROLLER, {}).get("cloud") != "localhost":
        raise LabError("Run just vm ingress-bootstrap first; expected ratings-lxd on localhost")
    models = juju_json("models", "--controller", CONTROLLER)["models"]
    if not any(m["short-name"] == "ratings-edge" for m in models):
        run(
            "juju", "add-model", "--controller", CONTROLLER, "--no-switch", "ratings-edge", "localhost",
            "--config", "default-base=ubuntu@24.04", "--config", "automatically-retry-hooks=false",
        )
    check_model(EDGE_MODEL, "iaas", "localhost")


def status(model):
    return juju_json("status", "--model", model, "--relations")


def check_apps(current, expected):
    for name, (charm, revision, _channel, _base, _trust) in expected.items():
        app = current["applications"].get(name)
        if app is None:
            if name in current.get("application-endpoints", {}):
                raise LabError(f"{name} is already a consumed offer, not a local application")
            continue
        if app.get("charm-name") != charm or app.get("charm-rev") != revision:
            raise LabError(f"{name}: expected {charm} revision {revision}; refresh/reconcile explicitly")


def relation_present(current, left, right):
    # Juju 3.6 omits top-level relations from JSON, even with --relations.
    # All managed requirers here have exactly one intended provider application.
    app, endpoint = left.split(":")
    remote = right.split(":")[0]
    relations = current["applications"].get(app, {}).get("relations", {}).get(endpoint, [])
    providers = {item if isinstance(item, str) else item["related-application"] for item in relations}
    if providers - {remote}:
        raise LabError(f"Conflicting relation on {left}; refusing to replace it")
    return remote in providers


def deploy_apps(model, current, expected):
    for name, (charm, revision, channel, base, trust) in expected.items():
        if name not in current["applications"]:
            run(
                "juju", "deploy", "--model", model, charm, name,
                f"--revision={revision}", f"--channel={channel}", f"--base=ubuntu@{base}",
                "--num-units=1", *(["--trust"] if trust else []), timeout=300,
            )


def configure(model, apps):
    for app in apps:
        desired = CONFIG.get(app, {})
        if not desired:
            continue
        current = juju_json("config", "--model", model, app)["settings"]
        if app == "ingress-configurator" and current.get("external-grpc-port", {}).get("value"):
            raise LabError("Leave external-grpc-port unset; this lab uses edge TLS port 443")
        # Never silently undo an operator's hostname, routing, or rewrite changes.
        changed = []
        for key, value in desired.items():
            setting = current[key]
            existing = setting.get("value")
            if existing == value:
                continue
            if setting.get("source") != "default" and existing not in (None, ""):
                raise LabError(f"Conflicting {app} config {key}; reconcile explicitly")
            changed.append(f"{key}={str(value).lower() if isinstance(value, bool) else value}")
        if changed:
            run("juju", "config", "--model", model, app, *changed)


def offer(source, app, endpoint, interface, name, target, alias):
    controller, model = source.split(":")
    offers = juju_json("offers", "--model", source)
    current = status(target)
    if alias in current["applications"]:
        raise LabError(f"{alias} is already a local application")
    existing = current.get("application-endpoints", {}).get(alias)
    if name not in offers and existing is not None:
        raise LabError(f"Unrelated consumed offer at {alias}; source offer is missing")
    if name not in offers:
        run("juju", "offer", "--controller", controller, f"{model}.{app}:{endpoint}", name)
        offers = juju_json("offers", "--model", source)
    entry = offers[name]
    if (
        entry.get("application") != app
        or entry.get("endpoints", {}).get(endpoint, {}).get("interface") != interface
    ):
        raise LabError(f"Unrelated offer {name}; refusing to consume")
    url = f"{controller}:{entry['offer-url']}"
    if existing is None:
        run("juju", "consume", "--model", target, url, alias)
    elif existing.get("url") not in (url, entry["offer-url"]):
        raise LabError(f"Unrelated consumed offer at {alias}")


def integrate(model, relations):
    current = status(model)
    for left, right in relations:
        if not relation_present(current, left, right):
            run("juju", "integrate", "--model", model, left, right)


def deploy():
    check_model(MODEL, "caas", "k8s")
    current = status(MODEL)
    if current["applications"].get("ratings", {}).get("charm-name") not in RATINGS_CHARMS:
        raise LabError("Deploy the Ratings sidecar first (normal rollout is unchanged)")

    endpoint = current["applications"]["ratings"].get("endpoint-bindings", {})
    if "traefik-route" not in endpoint:
        raise LabError("Refresh Ratings with the optional traefik-route requirement before ingress-deploy")
    check_apps(current, K8S_APPS)
    for left, right in K8S_RELATIONS:
        relation_present(current, left, right)
    edge_model()
    edge = status(EDGE_MODEL)
    check_apps(edge, EDGE_APPS)
    for left, right in EDGE_RELATIONS:
        relation_present(edge, left, right)
    deploy_apps(MODEL, current, K8S_APPS)
    deploy_apps(EDGE_MODEL, edge, EDGE_APPS)
    configure(MODEL, ("ratings", *K8S_APPS))
    configure(EDGE_MODEL, EDGE_APPS)
    offer(EDGE_MODEL, "edge", "haproxy-route", "haproxy-route", "ratings-edge", MODEL, "edge")
    offer(MODEL, "self-signed-certificates", "send-ca-cert", "certificate_transfer",
          "ratings-traefik-ca", EDGE_MODEL, "traefik-ca")
    integrate(MODEL, K8S_RELATIONS)
    integrate(EDGE_MODEL, EDGE_RELATIONS)
    print("Ingress wired; run just vm ingress-wait (no automatic upgrade, DNS, ACME, or exposure)")


def wait(seconds):
    deadline = time.monotonic() + seconds
    previous = {}
    while time.monotonic() < deadline:
        ready = True
        for model, names in ((MODEL, ("ratings", *K8S_APPS)), (EDGE_MODEL, EDGE_APPS)):
            current = status(model)
            for app in names:
                # IAAS status omits scale; these machine applications are single-unit.
                if model == EDGE_MODEL and app in current["applications"]:
                    current["applications"][app].setdefault("scale", 1)
                active, summary = health(current, app)
                ready = ready and active
                if previous.get((model, app)) != summary:
                    print(f"{model}: {summary}", flush=True)
                    previous[model, app] = summary
        if ready:
            return
        time.sleep(min(5, max(0, deadline - time.monotonic())))
    raise LabError(f"Ingress readiness timed out after {seconds}s; inspect both models with ingress-status")


def edge_address():
    check_model(EDGE_MODEL, "iaas", "localhost")
    current = status(EDGE_MODEL)
    check_apps(current, EDGE_APPS)
    units = current["applications"]["edge"].get("units", {})
    if len(units) != 1:
        raise LabError("Expected exactly one local edge unit")
    address = next(iter(units.values()))["public-address"]
    ip = ipaddress.ip_address(address)
    if ip.version != 4 or not ip.is_private or ip.is_loopback or ip.is_unspecified:
        raise LabError("Expected a private LXD IPv4 edge address")
    return address


def ca_certificate():
    check_model(EDGE_MODEL, "iaas", "localhost")
    check_apps(status(EDGE_MODEL), EDGE_APPS)
    result = juju_json(
        "run", "--model", EDGE_MODEL, "--wait=60s", "edge-certificates/leader", "get-ca-certificate"
    )
    tasks = list(result.values())
    if len(tasks) != 1 or tasks[0].get("status") != "completed":
        raise LabError("CA export action did not complete")
    pem = tasks[0]["results"]["ca-certificate"].strip() + "\n"
    validate_ca(pem)
    return pem


def validate_ca(pem):
    if not re.fullmatch(r"-----BEGIN CERTIFICATE-----\s+[A-Za-z0-9+/=\s]+-----END CERTIFICATE-----\s*", pem):
        raise LabError("Expected exactly one public CA certificate, not a key or action dump")
    ssl.create_default_context(cadata=pem)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("bootstrap", "deploy", "wait", "endpoint", "ca"))
    parser.add_argument("--timeout", type=int, default=900)
    args = parser.parse_args()
    try:
        if args.timeout <= 0:
            raise LabError("Timeout must be positive")
        if args.command == "bootstrap":
            bootstrap()
        elif args.command == "deploy":
            deploy()
        elif args.command == "wait":
            wait(args.timeout)
        elif args.command == "endpoint":
            print(json.dumps({"address": edge_address(), "hostname": HOSTNAME}))
        elif args.command == "ca":
            print(ca_certificate(), end="")
    except (LabError, subprocess.CalledProcessError, OSError, ValueError, KeyError) as exc:
        print(f"Ingress failed: {exc}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
