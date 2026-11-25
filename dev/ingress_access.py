#!/usr/bin/env python3
"""Host-only CA export and loopback SSH forwarding through the Multipass VM."""

import argparse
import ipaddress
import json
from pathlib import Path
import re
import subprocess
import sys

from juju_ingress import HOSTNAME, validate_ca
from juju_rollout import LabError, atomic_write, run

ARTIFACTS = Path(__file__).resolve().parent / "build" / "ingress"
VM_HELPER = "/home/ubuntu/app-center-ratings/dev/juju_ingress.py"
# Multipass authenticates this public-key handoff; no VM/Juju private key is exported.
AUTHORIZE = '''import os, pathlib, sys, tempfile
key, address = sys.argv[1:]
root = pathlib.Path.home() / ".ssh"
root.mkdir(mode=0o700, exist_ok=True)
path = root / "authorized_keys"
if path.is_symlink():
    sys.exit("Refusing symlink authorized_keys")
lines = path.read_text().splitlines() if path.exists() else []
prefix = 'restrict,port-forwarding,permitopen="'
entry = prefix + address + ':443",command="/bin/false" ' + key
for line in lines:
    if key.split()[1] in line and not (line.startswith(prefix) and line.endswith(key)):
        sys.exit("Dedicated ingress key already has unmanaged authorization")
updated = [line for line in lines if not line.endswith(key)] + [entry]
if updated != lines:
    fd, temporary = tempfile.mkstemp(dir=root)
    try:
        with os.fdopen(fd, "w") as stream:
            stream.write("\\n".join(updated) + "\\n")
        os.replace(temporary, path)
    finally:
        pathlib.Path(temporary).unlink(missing_ok=True)
'''


def remote(vm, *command):
    return run(
        "multipass", "exec", "--no-map-working-directory", vm, "--", *command,
        capture=True, timeout=120,
    )


def export_ca(vm, artifacts=ARTIFACTS):
    pem = remote(vm, "env", "PATH=/snap/bin:/usr/bin:/bin", "python3", VM_HELPER, "ca")
    validate_ca(pem)
    path = artifacts / "edge-ca.pem"
    atomic_write(path, pem)
    print(f"Public edge CA written to {path}; not installed in the system trust store")


def tunnel(vm, port, artifacts=ARTIFACTS):
    info = json.loads(run("multipass", "info", vm, "--format=json", capture=True, timeout=30))["info"][vm]
    if info["state"] != "Running":
        raise LabError("VM is not running; run just up first")
    addresses = [ipaddress.ip_address(a) for a in info["ipv4"]]
    vm_ip = next((str(a) for a in addresses if a.version == 4 and a.is_private and not a.is_loopback), None)
    if vm_ip is None:
        raise LabError("No private VM IPv4 address")
    endpoint = json.loads(remote(vm, "env", "PATH=/snap/bin:/usr/bin:/bin", "python3", VM_HELPER, "endpoint"))
    address = ipaddress.ip_address(endpoint["address"])
    if (endpoint.get("hostname") != HOSTNAME or address.version != 4
            or not address.is_private or address.is_loopback or address.is_unspecified):
        raise LabError("Invalid local edge endpoint")
    artifacts.mkdir(parents=True, exist_ok=True, mode=0o700)
    key = artifacts / "tunnel-ed25519"
    if key.is_symlink() or key.with_suffix(".pub").is_symlink():
        raise LabError("Refusing symlink tunnel key")
    if not key.exists():
        if key.with_suffix(".pub").exists():
            raise LabError("Public key exists without private key; reconcile the ingress artifacts")
        run("ssh-keygen", "-q", "-t", "ed25519", "-N", "", "-C", "ratings-lab-ingress", "-f", key)
    if key.stat().st_mode & 0o077:
        raise LabError("Tunnel private key permissions must be 0600 or stricter")
    public = run("ssh-keygen", "-y", "-f", key, capture=True).strip()
    if not re.fullmatch(r"ssh-ed25519 [A-Za-z0-9+/=]+(?: ratings-lab-ingress)?", public):
        raise LabError("Invalid dedicated tunnel public key")
    # Normalize the comment for repeat runs (ssh-keygen -y may retain it).
    public = " ".join(public.split()[:2]) + " ratings-lab-ingress"
    remote(vm, "python3", "-c", AUTHORIZE, public, str(address))
    host_key = remote(vm, "cat", "/etc/ssh/ssh_host_ed25519_key.pub").strip().split()
    if len(host_key) < 2 or not re.fullmatch(r"[A-Za-z0-9+/=]+", host_key[1]) or host_key[0] != "ssh-ed25519":
        raise LabError("Invalid VM SSH host public key")
    known_hosts = artifacts / "known_hosts"
    atomic_write(known_hosts, f"{vm_ip} {host_key[0]} {host_key[1]}\n")
    print(f"Forwarding {HOSTNAME}:{port} via VM to HAProxy {address}:443; Ctrl-C to stop", flush=True)
    run(
        "ssh", "-T", "-N", "-i", key,
        "-o", "BatchMode=yes", "-o", "IdentitiesOnly=yes", "-o", "ForwardAgent=no",
        "-o", "ConnectTimeout=10", "-o", "ExitOnForwardFailure=yes",
        "-o", "ServerAliveInterval=15", "-o", "ServerAliveCountMax=3",
        "-o", "StrictHostKeyChecking=yes", "-o", f"UserKnownHostsFile={known_hosts}",
        "-o", "GlobalKnownHostsFile=/dev/null", "-o", "UpdateHostKeys=no",
        "-L", f"127.0.0.1:{port}:{address}:443", f"ubuntu@{vm_ip}", timeout=None,
    )


def tls_check(port, artifacts=ARTIFACTS):
    ca = artifacts / "edge-ca.pem"
    validate_ca(ca.read_text())
    output = run(
        "openssl", "s_client", "-4", "-connect", f"{HOSTNAME}:{port}", "-servername", HOSTNAME,
        "-verify_hostname", HOSTNAME, "-verify_return_error", "-CAfile", ca, "-alpn", "h2",
        capture=True, timeout=20,
    )
    if "ALPN protocol: h2" not in output or "Verify return code: 0 (ok)" not in output:
        raise LabError("Edge TLS verification or HTTP/2 ALPN failed")
    print("PASS: edge CA, hostname, and ALPN h2 verified")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("ca", "tunnel", "tls-check"))
    parser.add_argument("--vm", default="ratings-lab")
    parser.add_argument("--port", type=int, default=8443)
    args = parser.parse_args()
    try:
        if not re.fullmatch(r"[a-zA-Z][a-zA-Z0-9-]*", args.vm) or not 1024 <= args.port <= 65535:
            raise LabError("Expected a Multipass VM name and an unprivileged host port (1024-65535)")
        if args.command == "ca":
            export_ca(args.vm)
        elif args.command == "tunnel":
            tunnel(args.vm, args.port)
        else:
            tls_check(args.port)
    except (LabError, subprocess.CalledProcessError, OSError, ValueError, KeyError) as exc:
        print(f"Ingress access failed: {exc}", file=sys.stderr)
        return 1
    except KeyboardInterrupt:
        return 130
    return 0


if __name__ == "__main__":
    sys.exit(main())
