# Ratings charm-development lab

This guide builds a disposable **Ubuntu 24.04 amd64 VM** with Canonical Kubernetes, Juju, PostgreSQL, COS Lite, and Rust/charm/rock build tools.
It does **not** implement/deploy a Ratings charm, convert Compose to Kubernetes manifests, or add telemetry. Application integration is future work.

## Contents

- [Scope and safety](#scope-and-safety)
- [1. Install host tools and clone](#1-install-host-tools-and-clone)
- [2. Create and prepare the VM](#2-create-and-prepare-the-vm)
- [3. Check access and record versions](#3-check-access-and-record-versions)
- [4. Build and test Ratings](#4-build-and-test-ratings)
- [5. Deploy database and observability dependencies](#5-deploy-database-and-observability-dependencies)
- [6. Optional Kubernetes experiments](#6-optional-kubernetes-experiments)
- [7. Future charm implementation](#7-future-charm-implementation)
- [Daily workflow and recipe reference](#daily-workflow-and-recipe-reference)
- [Troubleshooting and references](#troubleshooting-and-references)

## Scope and safety

**Host requirements:** Ubuntu 24.04 on amd64, hardware virtualization enabled in
firmware, working KVM, and enough spare capacity for a VM with **4 CPUs, 12G RAM,
60G disk**. Leave resources for the host. Downloads need network access to Ubuntu
APT/image servers, the Snap Store, Charmhub, GitHub, Rust/Python package sources,
and container registries. No public-cloud account is needed; downloads consume
bandwidth and disk space.

**Command convention:** commands run on the **host, from the repository root**
unless a block says **VM**. The initial installation/clone commands necessarily
precede entering the repository. `just vm ...` runs a recipe inside the VM; it is
still a command you type on the host.

| File | Runs where | Responsibility |
| --- | --- | --- |
| [`../justfile`](../justfile) | Host | Multipass lifecycle, mounting, VM execution, dedicated kubeconfig, local forwarding |
| [`../justfile.vm`](../justfile.vm) | VM | Preparation, builds, Juju dependencies, Kubernetes experiments |
| [`../dev/concierge.yaml`](../dev/concierge.yaml) | VM via Concierge | Tool channels, providers, controller, Kubernetes features |

Concierge runs as root, initializes LXD, changes networking/iptables and group
membership, and **overwrites the VM user's kubeconfig**. LXD socket access is
root-equivalent; Kubernetes admin credentials and Juju `--trust` deployments are
privileged too. Do not run this preparation on your everyday host or a shared
cluster. The mounted checkout is writable from the VM: isolation does not protect
host source files from VM edits. Keep important work committed or backed up.

The existing [`../docker-compose.yml`](../docker-compose.yml) has three services:
`db` (PostgreSQL, port 5432), `ratings` (gRPC, port 8080), and `snapcraft-mock`
(port 11111). Ratings needs database credentials, JWT/admin configuration, and a
Snap Store API URL. It runs SQL migrations before starting the server.
[`../src/main.rs`](../src/main.rs) writes structured JSON logs to stdout; it does
not export OTLP traces or expose a Prometheus application-metrics endpoint.
Compose remains a separate development/integration-test workflow; Docker is not
a requirement for this VM lab, and these helpers do not start the Compose stack.

## 1. Install host tools and clone

Assumes snapd already installed.

**HOST, after cloning:** install the prerequisites:

```sh
sudo snap install just --classic --channel=latest/stable
just host-install
just
```

If a snap is already installed, skip its install command; use `sudo snap refresh
kubectl --channel=1.32/stable` to align an existing kubectl. If `/snap/bin` is not
on your PATH, reconnect your login shell or invoke `/snap/bin/just` explicitly.
Resolve a failed KVM check before launching the VM.

From the checkout, `just host-install` performs the host APT/snap installation;
it is useful for completing or repeating setup but cannot bootstrap `just` itself.
Juju, Kubernetes **server**, Concierge, LXD, Rust, and craft tools belong in the
VM, not on the host.

## 2. Create and prepare the VM

Defaults are configurable with environment variables. Set overrides **before**
creation, and keep the same VM name in later shells:

```sh
# Optional overrides; the values shown are the defaults.
export RATINGS_VM=ratings-lab
export RATINGS_VM_CPUS=4
export RATINGS_VM_MEMORY=12G
export RATINGS_VM_DISK=60G
just create
just bootstrap
just vm rust
```

`create` launches Ubuntu 24.04 if absent, or starts the existing VM, and mounts
this checkout at `/home/ubuntu/app-center-ratings` using Multipass's **classic
SSHFS mount**. This is the portability choice, not the native mount from some
upstream tutorials. Resource variables apply at launch; they do not resize an
existing VM. Do not replace or purge another VM to resolve a naming conflict.

`bootstrap` installs VM `ca-certificates`, `curl`, `git`, `python3`, `snapd`, and
`just` if needed, then runs `just -f justfile.vm prepare` in the mounted checkout.
Preparation installs these **VM APT packages**, including `rsync` for rock builds:

```text
build-essential ca-certificates curl git libssl-dev pkg-config
protobuf-compiler python3 python3-pip python3-venv skopeo jq rsync
```

Concierge additionally installs `gnome-keyring`, `python3-pip`, and `python3-venv`
from its configuration, and the Kubernetes provider installs `iptables` if missing.
These are VM packages too; you do not need to install them separately on the host.

Preparation installs Concierge if absent and executes **only the tracked configuration**:
`sudo concierge prepare --config dev/concierge.yaml`. Do not add `--preset`/`-p`:
this lab is not relying on undocumented preset/config merge behavior. In that
file, `host:` means **the VM on which Concierge runs**, not your physical host.

| VM snap/tool | Channel or installation | Purpose |
| --- | --- | --- |
| Concierge | `latest/stable`, classic | Apply the tracked development configuration |
| Juju | `3.6/stable` | Controller `ratings-k8s`, Kubernetes cloud `k8s`, initial `testing` model |
| Canonical Kubernetes (`k8s`) | `1.32-classic/stable` | Single-node cluster |
| LXD | `5.21/stable` | Craft builds; enabled with Juju bootstrap **false** |
| Charmcraft, Rockcraft, `astral-uv`, `just`, `rustup` | `latest/stable` | Craft, Python, task runner, Rust installer |
| Standalone kubectl | `1.32/stable`, classic | Refreshed after Concierge to match Kubernetes |
| tox | `uv tool install tox --with tox-uv` | Future Python charm tests |
| Rust toolchain | stable, minimal + rustfmt/clippy | Installed separately by `just vm rust` |

Concierge handles confinement requirements for configured tool snaps. Kubernetes
features are network, DNS, local storage, and an L2 load balancer with pool
`10.43.45.0/28`. That pool is a lab setting, **not a promise of host routing**.
A Multipass NAT network does not make arbitrary load-balancer addresses reachable
from your host. The API/kubeconfig and port-forward paths below avoid that issue.

### Manual VM preparation equivalent

Use this instead of the VM preparation recipe when inspecting the installation
step by step. Open `just shell`, then run the following **VM** commands:

```sh
cd /home/ubuntu/app-center-ratings
sudo apt-get update
sudo apt-get install -y build-essential ca-certificates curl git libssl-dev \
  pkg-config protobuf-compiler python3 python3-pip python3-venv skopeo jq rsync snapd
sudo systemctl enable --now snapd.socket
# On a fresh VM; skip either install if the snap is already present.
sudo snap install just --classic --channel=latest/stable
sudo snap install concierge --classic --channel=latest/stable
cat dev/concierge.yaml
sudo concierge prepare --config dev/concierge.yaml
sudo snap refresh kubectl --channel=1.32/stable
export PATH="/home/ubuntu/.cargo/bin:/home/ubuntu/.local/bin:/snap/bin:$PATH"
uv tool install tox --with tox-uv
rustup toolchain install stable --profile minimal --component rustfmt --component clippy
rustup default stable
exit
```

Reconnect with `just shell` after preparation so new group memberships take
effect, especially for LXD. The host's VM-execution helper adds `.cargo/bin`,
`.local/bin`, and `/snap/bin` to PATH for non-login execution; an old interactive
shell may still have stale PATH/groups. Preparation is not a cluster reset tool.

## 3. Check access and record versions

```sh
just info
just vm status
just vm versions
just kubeconfig
kubectl --kubeconfig "$HOME/.kube/ratings-local.yaml" get nodes
just kubectl get pods -A
just vm juju controllers
just vm juju models -c ratings-k8s
```

`kubeconfig` exports admin configuration using the VM IP and
`k8s config --server https://<VM-IP>:6443` into the separate host file
`~/.kube/ratings-local.yaml`. It does **not** overwrite `~/.kube/config` or change
its context. Prefer `just kubectl ...`, which selects that file explicitly.
Treat the exported file as a secret; do not commit it. Regenerate it after VM
recreation, restore, or an IP change. A custom VM name still uses this dedicated
file, so export again when switching labs; it is not a multi-VM kubeconfig store.

`status` checks the cluster/controller; inspect actual output rather than assuming
that preparation implies readiness. Save `just vm versions` and host `snap list
just multipass kubectl` output with experiment notes. Channels, Rust stable, bundle
contents, and dependencies can change; channels are **not immutable revision
pins**. The guide's CLI/config references were checked against official docs
(including Concierge v1.9.0); this is not a claim of a runtime-tested deployment.

For a reusable baseline:

```sh
just snapshot prepared
# Snapshot stops the VM and leaves it stopped.
just up
```

A VM snapshot does not back up the host-mounted checkout. The VM's local storage
is a disposable single-node lab, not production HA storage or a database backup.

## 4. Build and test Ratings

```sh
just vm build
just vm test
just vm rock
```

`build` runs `cargo build --release`; `test` runs `cargo test --lib`, with
`CARGO_TARGET_DIR` under `/home/ubuntu/.cache/ratings/target` rather than the
host-mounted `target/`. Database-feature tests are ignored by default; passing
unit tests does not verify a working database, gRPC deployment, or integration
suite. See [`../CONTRIBUTING.md`](../CONTRIBUTING.md) for the existing Compose tests.

`rock` copies sources into VM-local storage with rsync, excluding `.git`, root and
mock build targets, generated `.rock`/`.charm` archives, and `.local/`, then runs `rockcraft pack`. This avoids building
directly over SSHFS. Inspect the recipe/output for its artifact location.
[`../rockcraft.yaml`](../rockcraft.yaml) already targets amd64, uses `base: bare`
and `build_base: ubuntu@24.04`, runs as `_daemon_`, and defines an enabled Pebble
service named `ratings` executing `/bin/ratings`. A packed `.rock` is **not** a
`.charm`, a published registry image, or an application deployment. Image export,
registry publication and charm OCI-resource wiring remain future work.

## 5. Deploy database and observability dependencies

```sh
just vm models
just vm postgres
just vm cos
just vm juju status -m ratings-k8s:ratings-dev --relations
just vm juju status -m ratings-k8s:cos-lite --relations
```

`models` idempotently creates `ratings-dev` and `cos-lite` on cloud `k8s` using
controller `ratings-k8s`. `postgres` checks for an existing application named
`postgresql`, otherwise deploys **one unit** of `postgresql-k8s`, channel `16/stable`, with `--trust`
to `ratings-dev`. This does not create Ratings database relations or configure
the Ratings server. Initial downloads/storage provisioning can take time.

`cos` deploys the existing **`cos-lite` `latest/stable` Charmhub bundle**, with
`--trust`, into `cos-lite`. Repeating bundle deployment can reconcile changes;
it is not a guaranteed no-op and does not pin every charm revision. This bundle
is an intentional simple lab path. For a new long-lived COS deployment, prefer
the [current Terraform tutorial](https://canonical.com/observability/docs/track-3.0/tutorial/cos-lite-canonical-k8s-sandbox/).
Terraform is optional and is not installed or required by these helpers. Do not
apply an independent Terraform deployment over this bundle-managed model without
planning ownership/migration.

Once Grafana is ready:

```sh
just vm grafana-password
just vm cos-endpoints
just kubeconfig
just grafana
```

`grafana-password` runs Grafana's admin-password action; treat the output as a
secret. `cos-endpoints` runs Traefik's endpoint-discovery action. Discovered URLs
may be reachable only inside the VM/cluster; verify reachability instead of
adding guessed host routes.

`just grafana` runs a **foreground host-local** kubectl port-forward to
`svc/grafana` in namespace `cos-lite`, using the dedicated kubeconfig. Leave it
running, visit **http://127.0.0.1:3000/cos-lite-grafana/**, and sign in as `admin`
with the retrieved password. Keep the `/cos-lite-grafana/` prefix: Grafana remains
configured for the bundle's ingress path even when accessed through a port-forward.
Stop with Ctrl-C. No load-balancer route, host SSH key, or public listener is needed. VM-only `just -f justfile.vm cos-forward` instead forwards to the VM's
localhost; that is **not** your host's localhost. Avoid `--address=0.0.0.0`: it
exposes the port to other machines and is only appropriate on a trusted lab network.

COS Lite supplies monitoring/logging components and Grafana; **it does not include
Tempo by default**. Tracing requires a separate backend/collector integration and
application instrumentation. A Juju relation or collector configuration alone
cannot create metrics or distributed traces that Ratings never emits.

## 6. Optional Kubernetes experiments

```sh
just vm metrics-server
just vm kubectl top nodes
just vm sandbox
just kubectl get namespace ratings-sandbox --show-labels
```

Metrics Server uses pinned upstream **v0.8.0** assets (compatible with Kubernetes
1.32), applies them and waits with a timeout. It provides Kubernetes CPU/memory
resource metrics, not application monitoring or a Prometheus-to-HPA adapter.
If readiness fails, inspect `just vm kubectl -n kube-system logs
deployment/metrics-server` and pod events. Only if you have diagnosed kubelet
certificate validation errors and accept the disposable-lab tradeoff, run:

```sh
just vm metrics-server-local-tls
just vm kubectl top nodes
```

That explicit opt-in appends `--kubelet-insecure-tls` and verifies rollout. It
weakens kubelet TLS verification; it is not automatic and is not production advice.
Prefer correcting certificate trust outside the lab.

`sandbox` creates `ratings-sandbox` with Pod Security Admission **restricted
warn/audit** labels, not enforcement. It deploys no workload and provides no
NetworkPolicy/RBAC isolation by itself. Do not apply namespace-wide restricted
enforcement to Juju/COS workloads without testing their sidecars and permissions.

## 7. Future charm implementation

Choose ownership before writing deployment resources:

- **Full Kubernetes charm, for testing the eventual Juju deployment:** Juju owns Ratings units, the workload
  OCI resource, Pebble lifecycle, database/observability relations, configuration,
  secrets and operational actions. Keep scaling under Juju's control.
- **Hybrid, recommended when native HPA testing is the priority:** Juju manages PostgreSQL/COS while you own a separate
  Kubernetes Deployment (for example in `ratings-sandbox`). You must implement
  database credentials, endpoint/secret delivery, migrations, health checks,
  observability and lifecycle yourself. Use a workload-side collector or suitable
  node collector for logs. Native HPA can own this Deployment's replicas with
  resource requests and Metrics Server; custom metrics additionally need a
  custom/external metrics adapter. Prometheus being present is not sufficient.

Do **not** attach an HPA to Juju-managed replicas assuming both controllers will
cooperate. After a custom charm is implemented and deployed as application
`ratings` in `ratings-dev`, `just vm scale 3` calls Juju's `scale-application`.
That is manual Juju scaling, not autoscaling; it cannot work before the app exists.

Optional scaffolding commands, **not part of environment bootstrap**:

```sh
just vm charm-init charm
# Implement and test the generated project before treating its package as Ratings.
just vm charm-pack charm
```

`charm-init` creates an empty directory and refuses a nonempty one before invoking
`charmcraft init --name ratings-k8s --project-dir charm`. It writes a generic
Kubernetes charm scaffold into the mounted checkout; no Ratings integration is
provided. `charm-pack` packs the specified project, not a pre-existing Ratings
charm. Read the generated project's tox configuration before running its tests.

Implementation checkpoints: use the rock as an OCI resource; wait for database
relation data; pass required `APP_*` configuration securely; coordinate migrations
before serving across replicas; add real readiness/liveness checks; support the
mock API separately for tests; and configure gRPC-aware TLS/ingress. Grafana is an
observability UI, not a Ratings frontend. An ordinary HTTP reverse proxy is not
automatically a correct gRPC ingress.

Add Prometheus metrics and/or Rust OpenTelemetry instrumentation explicitly.
Forward JSON stdout logs through a collector to Loki, redact credentials and
personal data, and add dashboards/alerts. Cross-model offers/integrations need
implemented charm endpoints. For the hybrid path, follow the [uncharmed workload
integration guide](https://canonical.com/observability/docs/track-3.0/how-to/integrate/integrating-cos-lite-with-uncharmed-applications/).
Test NetworkPolicies with DNS, database, Snap Store egress, observability and
**Juju controller/API connectivity** in mind; policies that sever agent/controller
access can break operations. None of these integrations/policies/manifests are
created by this setup.

## Daily workflow and recipe reference

```sh
just up
just vm status
just vm build
just vm test
just shell
# VM: exit to return to the host.
just stop
```

| Host command | Effect |
| --- | --- |
| `just host-install` | Install host prerequisites after `just` is available |
| `just create` / `just bootstrap` | Launch/start + classic mount / prepare VM |
| `just up` / `just stop` / `just shell` | Bring lab up / stop it / open VM shell |
| `just vm <recipe> [args...]` | Execute `justfile.vm` with VM tool PATH |
| `just kubeconfig` / `just kubectl ...` | Export dedicated admin file / use it |
| `just info` / `just grafana` | Inspect VM / foreground host localhost forwarding |
| `just snapshot [name]` | Default `prepared`; stop, snapshot, **remain stopped** |
| `just restore <name>` | Prompt for destructive restore; stop/restore/restart |
| `just delete` | Prompt; delete **only configured VM** with `--purge`, no global purge |

VM recipes: `prepare`, `rust`, `status`, `versions`, `build`, `test`, `rock`, `models`, `postgres`, `cos`,
`grafana-password`, `cos-endpoints`, `cos-forward`, `metrics-server`, `metrics-server-local-tls`, `sandbox`, `kubectl ...`, `juju ...`,
`charm-init [directory=charm]`, `charm-pack [directory=charm]`, `scale [units=3]`.
Inside the VM use `just -f justfile.vm <recipe>`; bare `just` selects the host Justfile. Inspect recipes with host `just --list` or `just vm default`.

Restore discards post-snapshot VM state, including database data and credentials, but does not restore mounted sources. After `just restore prepared`, run `just kubeconfig` and check status.
Deletion permanently removes the selected VM's state/snapshots; verify `RATINGS_VM` first. Never use global `multipass purge` on a host with other VMs.

## Troubleshooting and references

- Mount failures: check `multipass info ratings-lab` (substitute your configured name), rerun `just create`, and inspect host Multipass mount support rather than silently switching providers. If mounts were disabled by local policy, review the security implications before enabling `multipass set local.privileged-mounts=true`.
- Build-provider failures: reconnect groups and inspect VM `lxc info` and `lxc profile show default`. Normal Linux-container craft builds do not need nested hardware virtualization; a deliberately selected VM build provider may.
- API failures: use `just info`, regenerate kubeconfig, and check VM IP/firewall/VPN access to TCP 6443. Do not disable API TLS verification.
- Pending pods: inspect events/storage/resources with `just kubectl get pods -A` and `just kubectl -n <namespace> describe pod <pod-name>` using actual names.

Official references: [Ops environment tutorial](https://documentation.ubuntu.com/ops/latest/tutorial/from-zero-to-hero-write-your-first-kubernetes-charm/set-up-your-development-environment/), [Concierge configuration](https://canonical.com/juju/docs/concierge/reference/configuration/) ([v1.9.0 schema source](https://github.com/canonical/concierge/blob/v1.9.0/docs/reference/configuration.md)).
Also: [Canonical Kubernetes commands](https://documentation.ubuntu.com/canonical-kubernetes/latest/snap/reference/commands/), [Charmcraft init](https://documentation.ubuntu.com/charmcraft/latest/reference/commands/init/), and [Metrics Server v0.8.0 requirements](https://github.com/kubernetes-sigs/metrics-server/blob/v0.8.0/README.md).
