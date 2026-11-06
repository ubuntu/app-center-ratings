# Ratings local Juju deployment lab

This guide uses a disposable **Ubuntu 24.04 amd64 VM** with Canonical Kubernetes,
Juju, PostgreSQL, optional COS Lite, and Rust/charm/rock build tools. The Ratings
sidecar charm is implemented in [`../charm/`](../charm/): Juju owns the workload,
database relation, configuration, shared JWT secret, and scaling. This is a local
plaintext gRPC lab, not a production ingress or high-availability deployment.

If the local environment is already installed, skip to
[Deploy Ratings and use the API](#7-deploy-ratings-and-use-the-api):
`just vm rollout` builds and deploys or refreshes Ratings through Juju, then waits
for readiness. The default rollout also integrates logs with an existing COS
Lite deployment; a no-COS rollout path is documented there too. Preparing the VM
alone does not deploy Ratings.

## Contents

- [Scope and safety](#scope-and-safety)
- [1. Install host tools and clone](#1-install-host-tools-and-clone)
- [2. Create and prepare the VM](#2-create-and-prepare-the-vm)
- [3. Check access and record versions](#3-check-access-and-record-versions)
- [4. Build and test Ratings (optional)](#4-build-and-test-ratings-optional)
- [5. Database and optional observability dependencies](#5-database-and-optional-observability-dependencies)
- [6. Optional Kubernetes experiments](#6-optional-kubernetes-experiments)
- [7. Deploy Ratings and use the API](#7-deploy-ratings-and-use-the-api)
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
| [`../justfile.vm`](../justfile.vm) | VM | Preparation, image/charm builds, Juju applications and relations, Kubernetes experiments |
| [`../dev/concierge.yaml`](../dev/concierge.yaml) | VM via Concierge | Tool channels, providers, controller, Kubernetes features |
| [`../dev/juju_rollout.py`](../dev/juju_rollout.py) | VM via recipes | Loopback registry, image publication, deploy/refresh, cross-model logging, bounded readiness polling |

Concierge runs as root, initializes LXD, changes networking/iptables and group
membership, and **overwrites the VM user's kubeconfig**. LXD socket access is
root-equivalent; Kubernetes admin credentials and Juju `--trust` deployments are
privileged too. Do not run this preparation on your everyday host or a shared
cluster. The mounted checkout is writable from the VM: isolation does not protect
host source files from VM edits. Keep important work committed or backed up.

The existing [`../docker-compose.yml`](../docker-compose.yml) has three services:
`db` (PostgreSQL, port 5432), `ratings` (gRPC, port 8080), and `snapcraft-mock`
(port 11111). Ratings needs database credentials, a JWT signing secret, and a
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
| tox | `uv tool install tox --with tox-uv` | Python test tooling; see the charm's own test instructions |
| Rust toolchain | stable, minimal + rustfmt/clippy | Installed separately by `just vm rust` |

Concierge handles confinement requirements for configured tool snaps. Kubernetes
features are network, DNS, local storage, and an L2 load balancer with pool
`10.43.45.0/28`. That pool is a lab setting, **not a promise of host routing**.
A Multipass NAT network does not make arbitrary load-balancer addresses reachable
from your host. The API/kubeconfig and port-forward paths below avoid that issue.

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

Export prints a progress message before VM inspection and before credential export.
Each step has a 30-second deadline (plus up to 5 seconds for forced termination).
It runs outside the mounted checkout with stdin closed and `sudo -n`, so it cannot
wait for a password prompt. Failed, timed-out, or empty exports leave an existing
kubeconfig untouched. If a healthy but slow VM needs more time, use
`RATINGS_KUBECONFIG_TIMEOUT=60s just kubeconfig`.

The export uses a pipe (`multipass ... | cat > temporary-file`) rather than
redirecting Multipass directly to a host file. With snapped `just` and Multipass,
that direct redirection can trigger an AppArmor `file_inherit` denial in
`snap-confine` and leave the export hanging. The pipe avoids passing the writable
file descriptor into Multipass; `pipefail` still rejects failed or partial exports.
No AppArmor changes or snap confinement overrides are required.

`status` checks the cluster/controller; inspect actual output rather than assuming
that preparation implies readiness. Save `just vm versions` and host `snap list
just multipass kubectl` output with experiment notes. Channels, Rust stable, bundle
contents, and dependencies can change; channels are **not immutable revision
pins**. The guide's CLI/config references were checked against official docs
(including Concierge v1.9.0). The implemented rollout was also exercised in the
local lab; see the validation scope at the end of section 7.

For a reusable baseline:

```sh
just snapshot prepared
# Snapshot stops the VM and leaves it stopped.
just up
```

A VM snapshot does not back up the host-mounted checkout. The VM's local storage
is a disposable single-node lab, not production HA storage or a database backup.

## 4. Build and test Ratings (optional)

```sh
just vm build
just vm test
just vm charm-test
just vm rock
```

`charm-test` installs the locked development dependencies into a VM-local venv
under `~/ratings-lab-artifacts/test-venv`, then runs the charm's pytest suite and
Ruff lint. Dependency installation needs network access; the tests themselves
use fake Juju/Pebble data, not a live deployment.

`build` runs `cargo build --release`; `test` runs `cargo test --lib`, with
`CARGO_TARGET_DIR` under `/home/ubuntu/.cache/ratings/target` rather than the
host-mounted `target/`. Database-feature tests are ignored by default; passing
unit tests does not verify a working database, gRPC deployment, or integration
suite. See [`../CONTRIBUTING.md`](../CONTRIBUTING.md) for the existing Compose tests.

`rock` copies sources into `/home/ubuntu/.cache/ratings/rock-src` with rsync,
excluding `.git`, root and mock build targets, generated `.rock`/`.charm`
archives, `.local/`, and `charm/`, then runs `rockcraft pack`. This avoids building
directly over SSHFS. The full rollout also builds the rock, so a separate `rock`
invocation is optional.
[`../rockcraft.yaml`](../rockcraft.yaml) targets amd64, uses `base: bare`
and `build-base: ubuntu@24.04`, runs as `_daemon_`, and defines an enabled Pebble
service named `ratings` executing `/bin/ratings`. A packed `.rock` is **not** a
`.charm`, a published registry image, or an application deployment. Deployment
must also publish an image reachable by Kubernetes and supply it as the charm's
`ratings-image` OCI resource; see [section 7](#7-deploy-ratings-and-use-the-api).

## 5. Database and optional observability dependencies

PostgreSQL is required for Ratings; COS Lite is optional for the charm. The
full `rollout` recipe creates/reuses PostgreSQL but expects COS Lite to exist
for its logging step. Skip an existing COS deployment rather than redeploying
the bundle on every application change. These commands manage the dependencies
separately from the Ratings image/charm deployment:

```sh
just vm models
just vm postgres
just vm cos
just vm juju status -m ratings-k8s:ratings-dev --relations
just vm juju status -m ratings-k8s:cos-lite --relations
```

`models` idempotently creates `ratings-dev` and `cos-lite` on cloud `k8s` using
controller `ratings-k8s`. It checks `juju models --controller ratings-k8s --format=json`,
reuses existing models, stops if that lookup fails, and creates missing models
with `--no-switch` to preserve your active model.

`postgres` checks for an existing application named `postgresql`, otherwise deploys **one unit** of `postgresql-k8s`, channel `16/stable`, with `--trust`
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

Ratings implements the optional **`logging`** endpoint (`loki_push_api`). After
Ratings and COS Lite are deployed, connect the two models with:

```sh
just vm ratings-observability
```

This recipe expects application **`loki`** (`loki-k8s`) in
`ratings-k8s:cos-lite`. It creates/reuses the **`ratings-logs`** offer for
`loki:logging`, consumes its actual offer URL in `ratings-k8s:ratings-dev` as
**`ratings-loki`**, and integrates **`ratings:logging ratings-loki:logging`**.
It refuses to reuse an unrelated offer or a consumer alias pointing elsewhere.
It does not deploy COS, and the full `rollout` already invokes this step.

For this NAT-based lab, it also removes **only Loki's** `loki:ingress` relation
to `traefik:ingress-per-unit`. Otherwise Loki advertises the L2 load-balancer
address, which workload pods cannot reliably reach. Loki then advertises its
internal `loki-0.loki-endpoints.cos-lite.svc.cluster.local:3100` endpoint instead.
This deliberately removes Loki's external ingress route, not its Grafana data
source or other COS ingress relations. Do not apply this lab-specific change to
a shared production COS model. Reapplying the COS bundle can restore the ingress
relation; rerun `ratings-observability` afterwards to restore the internal path.

`ratings:database` is not a logging endpoint. The charm uses native Pebble log
forwarding with Juju model/application/unit labels, not an extra Promtail sidecar.
Relating logging sends workload stdout/stderr to COS: consider the privacy impact
and redact sensitive logs before sharing them. Removing the relation disables
forwarding. The charm does not configure trust for private-CA Loki HTTPS.

COS Lite supplies monitoring/logging components and Grafana; **it does not include
Tempo by default**. Tracing requires a separate backend/collector integration and
application instrumentation. A Juju relation or collector configuration alone
cannot create metrics or distributed traces that Ratings never emits.

## 6. Optional Kubernetes experiments

These are not prerequisites for Ratings rollout and do not autoscale its
Juju-managed units. Keep native Deployment/HPA experiments separate from the
Ratings application.

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

## 7. Deploy Ratings and use the API

### Roll out from the existing environment

Reuse your prepared VM and the same `RATINGS_VM` setting. Do not rerun bootstrap,
recreate the VM, or redeploy an existing COS bundle just to deploy Ratings. The
default path assumes the Kubernetes/Juju environment is installed and COS Lite
already has application `loki` in `ratings-k8s:cos-lite`:

```sh
just up
just vm rollout
just vm ratings-status
just kubeconfig
just ratings
```

`just ratings` stays in the foreground; leave that terminal open for the
[gRPC smoke check](#host-local-grpc-smoke-check) in another host terminal. If COS
is absent and you want logging, run `just vm cos` once and inspect its status
before rollout. PostgreSQL need not be deployed separately: rollout creates the
models and the one-unit `postgresql` application if absent, or reuses them.

The full recipe runs these stages in order:

| Stage | Effect |
| --- | --- |
| `postgres` | Create/reuse models and PostgreSQL; do not automatically upgrade an existing provider |
| `rock` | Build the Ratings workload rock from VM-local sources |
| `charm-pack` | Pack the tracked Ratings operator from VM-local sources |
| `ratings-image` | Run `ratings-registry`, publish the rock, and record a digest-pinned OCI reference |
| `ratings-deploy` | Deploy one Ratings unit, or refresh the existing application; integrate its database |
| `ratings-observability` | Create/reuse the cross-model Loki logging integration; do not deploy COS |
| `ratings-wait` | Poll Ratings and PostgreSQL readiness with a default **600-second** deadline |

Builds and earlier stages are outside that readiness deadline. The waiter checks
that both applications are Active, their unit counts match the requested scale,
and all units are Active with Idle agents. A unit hook error aborts promptly;
individual Juju CLI calls also have bounded timeouts. It does not certify Loki
delivery, a successful gRPC request, or Snap Store connectivity. On failure,
inspect rather than blindly rerunning the whole build:

```sh
just vm ratings-status
just vm ratings-logs 100
# After diagnosing/fixing the issue, wait again without rebuilding:
just vm ratings-wait 600
```

`ratings-logs` shows the last 100 workload-container log lines by default, not
Juju agent logs or all replicas. A deliberately longer positive timeout, such as
`just vm ratings-wait 900`, changes only that readiness wait.

**Without COS:** the charm's logging relation is optional, but the full `rollout`
recipe always tries to create it. To deploy without COS, run the same stages
explicitly and omit `ratings-observability`:

```sh
just vm postgres
just vm rock
just vm charm-pack
just vm ratings-image
just vm ratings-deploy
just vm ratings-wait 600
```

You can add COS later and run `just vm ratings-observability` without rebuilding
Ratings.

### Image registry, build paths, and refreshes

All paths below are **inside the VM**, not in the host's mounted checkout:

| Path | Purpose |
| --- | --- |
| `/home/ubuntu/.cache/ratings/rock-src` | VM-local rock build tree and `.rock` archive |
| `/home/ubuntu/.cache/ratings/charm-src` | VM-local charm build tree and native pack output |
| `/home/ubuntu/ratings-lab-artifacts/ratings_amd64.charm` | Copied deployment archive readable by snapped Juju |
| `/home/ubuntu/ratings-lab-artifacts/image.json` | Published image reference, source rock path, and archive checksum |
| `/var/lib/ratings-registry` | Persistent VM-local registry image storage |

The non-hidden **`~/ratings-lab-artifacts`** directory is intentional: strictly
confined Juju cannot read deployment archives under `~/.cache`. Do not move the
handoff back into `.cache` or disable snap confinement to work around that.
`charm-pack` defaults to the existing `charm/` project and runs
`sudo -n charmcraft pack --destructive-mode --platform amd64` from its VM-local
copy. This native build can install APT development dependencies on the disposable
VM; it does not install them on the host. Do not run that destructive-mode build
on your workstation. `charm-init` is only for unrelated starter projects, not a
step for building this already-implemented Ratings charm.

`ratings-registry` installs the VM's **`docker-registry` APT package**, not a
registry container or Docker daemon. During package installation it masks the
default `docker-registry.service` to prevent an all-interface listener. It manages
**`ratings-registry.service`**, configured under `/etc/ratings-registry`, bound
only to the **VM's `127.0.0.1:5000`**. This is not the host's localhost or a pod's
localhost. Images have no registry authentication or TLS, so loopback-only
exposure is essential; do not open it to the network. Images remain in VM-local
storage across service restarts; this is not an external image archive or backup.

The helper requires **one Kubernetes node** and the existing containerd
`config_path=/etc/containerd/hosts.d`. It writes only the scoped HTTP registry
entry `/etc/containerd/hosts.d/127.0.0.1:5000/hosts.toml`, without restarting
containerd or rewriting its main config. It refuses to overwrite unmanaged
configuration. A multi-node cluster needs a shared registry instead of this
node-loopback design.

`ratings-image` uses **skopeo** to copy the OCI `.rock` archive, preserve its
digests, inspect the published manifest, and write
`127.0.0.1:5000/ratings@sha256:...` to `image.json`. The digest-pinned reference,
not a mutable tag, is passed as **`ratings-image`** to Juju. With no argument,
`just vm ratings-image` requires exactly one `ratings_*_amd64.rock` in
`rock-src`; if several exist, pass the desired **VM-side** archive path as its
positional argument. Publication does not rebuild the rock.

`just vm ratings-deploy` does **no builds**: it requires the packaged charm and
`image.json` above, verifies the image is present in the registry, then deploys
or refreshes `ratings` using that archive and OCI resource. It requires the
existing `postgresql` provider, reuses the expected database relation, and
refuses to replace a different database relation or an unrelated application.
For subsequent changes, choose only the stages needed:

```sh
# Rust/application or rock changes: rebuild and publish the image, then refresh.
just vm rock
just vm ratings-image
just vm ratings-deploy
just vm ratings-wait 600
```

```sh
# Charm-only changes: repack the operator, keeping the recorded image.
just vm charm-pack
just vm ratings-deploy
just vm ratings-wait 600
```

The image-only path still requires a previously packed charm; the charm-only
path requires an existing `image.json` and published image. Both use Juju refresh
with `--path` and `--resource` when the application exists. The full
`just vm rollout` is also repeatable, but rebuilds both artifacts; it does not
upgrade PostgreSQL or redeploy the COS bundle. No path here promises a zero-downtime
upgrade or a reversible schema migration.

### Charm interfaces

Use the existing [`../charm/`](../charm/) project, not a generated starter charm.
It declares Ubuntu 24.04/amd64 and requires Juju **3.6.4+**. The workload container
and Pebble service are both named `ratings`; the image resource is
**`ratings-image`**. The image is not bundled inside the `.charm` archive.
See [`../charm/README.md`](../charm/README.md) for packing, tests, and the full
operator lifecycle.

| Charm endpoint | Purpose |
| --- | --- |
| `database` | Required `postgresql_client` relation; dedicated `ratings` database and relation-scoped user |
| `logging` | Optional `loki_push_api` relation; Pebble stdout/stderr forwarding to Loki |
| `ratings-peers` | Shared application JWT secret reference and rotation notification |

The lab PostgreSQL application is named **`postgresql`**, not `postgresql-k8s`
(the charm's name). `ratings-deploy` creates this relation when absent; the
corresponding Juju command, if managing the integration manually, is:

```sh
just vm juju integrate -m ratings-k8s:ratings-dev \
  ratings:database postgresql:database
```

Do not copy a PostgreSQL admin password or invent a connection string: Juju
supplies relation-scoped credentials. Removing the database relation stops the
workload rather than leaving stale credentials in use.

### Configuration, readiness, and migrations

- `snap-store-url` defaults to `https://api.snapcraft.io/v2/`; it must be an
  absolute HTTP(S) URL ending in `/`. This uses the public Snap Store, **not**
  the Compose mock. Do not set it to the host's localhost: inside the workload
  container that means the container itself.
- `rust-log` defaults to `info,sqlx=warn,hyper=warn,reqwest=warn`. To adjust it:

  ```sh
  just vm juju config -m ratings-k8s:ratings-dev ratings rust-log=debug
  ```

  Verbose SQL/client logging may reveal sensitive request data. Those logs are
  shared with COS when related; restore the default after debugging.
- The charm starts Ratings only after database credentials and the shared JWT
  secret are available. It checks that Pebble's service is running and a TCP
  readiness check on port **8080** has succeeded. This is listener readiness,
  not a continuous database transaction or a Snap Store health test.
- Ratings embeds its migrations and runs them before opening the listener.
  Locked SQLx **0.8.6** enables PostgreSQL advisory migration locks by default,
  so concurrent units serialize migrations through the database. No separate
  migration Job or charm migration leader is needed. Take a real PostgreSQL
  backup before schema changes you need to recover from; a VM snapshot is not
  a database backup strategy.

### Host-local gRPC smoke check

After Ratings is ready, export the host kubeconfig and leave the forward running
in a separate host terminal:

```sh
just kubeconfig
just ratings
```

`just ratings [port=8080]` forwards Service **`ratings`** in namespace
**`ratings-dev`**, using the dedicated host kubeconfig and binding only to host
loopback. The endpoint is **`127.0.0.1:8080` on the host**, speaking **plaintext
gRPC**. If that host port is occupied, `just ratings 18080` forwards
`127.0.0.1:18080` to the same Service port 8080; change the grpcurl address below
to match. It is not a web page or a REST API. This foreground port-forward creates
neither an ingress nor a public listener; it does not require routing the lab's
LoadBalancer pool through Multipass NAT. Stop it with Ctrl-C, and restart it if
the selected pod is replaced during refresh or scaling. Do not bind it to
`0.0.0.0` or expose this plaintext listener on an untrusted network. The charm
opens 8080/TCP but does not configure listener TLS or certificate relations.

For an automated smoke check, optionally install **uv on the host** using its
[official installation instructions](https://docs.astral.sh/uv/getting-started/installation/)
and run this in another host terminal while `just ratings` is running:

```sh
just grpc-smoke
```

This executes [`../dev/grpc_smoke.py`](../dev/grpc_smoke.py). uv downloads the
pinned gRPC Python tools into its own environment; the script generates clients
in a temporary directory, creates a random smoke-test identity, authenticates,
performs a database-backed request, checks authorization rejection, and deletes
the identity. It prints no tokens or database credentials and needs no gRPC
reflection. An alternate forward can be checked with
`just grpc-smoke 127.0.0.1:18080`. uv is optional and is not installed by
`just host-install`; your host application can connect without it.

Alternatively, install **grpcurl on the host** using the
[upstream installation instructions](https://github.com/fullstorydev/grpcurl#installation)
or the matching Linux amd64 binary from its
[releases](https://github.com/fullstorydev/grpcurl/releases), and put it on your
PATH. It is not installed by `just host-install` or VM preparation. The server
does **not** enable gRPC reflection, so supply the repository's actual protos
with `-import-path proto -proto ...`; reflection-based `grpcurl list` is not a
health check for this service.

From the repository root, authenticate without a Bearer header:

```sh
grpcurl -plaintext -max-time 10 \
  -import-path proto -proto ratings_features_user.proto \
  -d '{"id":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"}' \
  127.0.0.1:8080 ratings.features.user.User/Authenticate
```

The request field is **`id`**, not `client_id`, and the application requires
exactly **64 bytes** (use a 64-character ASCII value). The proto describes a
SHA-256 client identifier. This fixed example is only a disposable smoke-test
identity; Authenticate creates or updates its database user and returns a
`token`. It is not proof of a real person's identity. Do not use real identifying
data in shared examples or paste tokens into issue reports.

All other RPCs require metadata **`authorization: Bearer <token>`**. In a **host
Bash shell**, paste the returned token at the prompt rather than into shell
history, then make a read-only authenticated query:

```bash
read -r -s -p 'Paste the returned token: ' RATINGS_TOKEN
printf '\n'
grpcurl -plaintext -max-time 10 \
  -import-path proto -proto ratings_features_app.proto \
  -H "authorization: Bearer $RATINGS_TOKEN" \
  -d '{"snap_ids":["ratings-lab-smoke-check"]}' \
  127.0.0.1:8080 ratings.features.app.App/GetBulkRatings
unset RATINGS_TOKEN
```

An empty result (normally `{}` with default grpcurl output) is expected if there
are no votes for that dummy snap ID. This exercises authentication and a database
read without submitting a vote or requiring Snap Store data for an empty result;
it does not validate Snap Store reachability. For real rating queries, use an
actual Snap Store **snap ID**, not just the snap name. JWTs currently expire after
one day; authenticate again after expiry or deliberate key rotation. The token
is still present in grpcurl's process arguments while the call runs, so use this
only on a trusted local workstation.

### Scaling and JWT rotation

```sh
just vm scale 3
just vm ratings-wait 600
just vm ratings-status
```

`scale` calls **`juju scale-application`** for `ratings`; it is manual
Juju-managed scaling, **not HPA/autoscaling**. Do not attach an HPA or run
`kubectl scale` against Juju-managed replicas: competing controllers can undo
one another's changes. Metrics Server and COS do not make this app autoscale.
Native HPA experiments belong in a separately owned `ratings-sandbox`
Deployment with its own lifecycle and credentials; this setup creates no such
workload. Scaling Ratings does not make the one-unit database or single-node
storage highly available.

The leader creates one application-owned Juju JWT signing secret, shared by all
units through `ratings-peers`. Ordinary restarts, charm refreshes, scale changes,
and leader changes reuse it, so authentication is not tied to one replica.
To rotate deliberately on the current leader:

```sh
just vm juju run -m ratings-k8s:ratings-dev ratings/leader rotate-jwt-secret
```

Rotation returns no secret, restarts units as peer notifications propagate, and
**invalidates existing JWTs**. There is no dual-key grace period, so requests may
fail during propagation; authenticate again afterwards. Deleting the published
secret is not a rotation mechanism and stops the workload. Treat database/JWT
values in Pebble's environment as sensitive: do not share unredacted environment
dumps, Pebble plans, or debug logs.

**Validation scope:** the complete `just vm rollout` succeeded against the local
Juju 3.6.29 / Kubernetes 1.32 lab, including an image refresh. Host-side gRPC
Authenticate, an authenticated database read, rejection without authorization,
and smoke-identity cleanup passed. Two units reached Active/Idle and a JWT issued
by one was accepted by the other; scaling back to one also passed. A Loki query
returned Ratings workload logs. The charm's 88 offline tests passed with 99%
coverage, including all 88 passing in the Ubuntu 24.04 VM; rollout orchestration
has 17 passing stdlib regression tests. This is
not production certification: external Snap Store queries, listener TLS,
NetworkPolicy enforcement, automatic scaling, and failure recovery under load
were not validated.

## Daily workflow and recipe reference

```sh
just up
just vm status
just vm ratings-status
just vm ratings-logs
# After changing source, choose the image/charm refresh stages in section 7.
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
| `just info` | Inspect the VM |
| `just ratings [port=8080]` / `just grafana` | Foreground host-loopback gRPC / Grafana forwarding |
| `just grpc-smoke [target=127.0.0.1:8080]` | Optional host-uv gRPC check and identity cleanup |
| `just vm rollout` | Build/publish/deploy or refresh Ratings; integrate existing COS; bounded readiness wait |
| `just vm ratings-status` / `just vm ratings-logs [lines=100]` | Ratings relations/status / recent workload logs |
| `just vm ratings-wait [seconds=600]` | Wait for Ratings/PostgreSQL Active/Idle units without rebuilding |
| `just vm scale N` | Set Ratings' requested unit count through Juju, not HPA |
| `just snapshot [name]` | Default `prepared`; stop, snapshot, **remain stopped** |
| `just restore <name>` | Prompt for destructive restore; stop/restore/restart |
| `just delete` | Prompt; delete **only configured VM** with `--purge`, no global purge |

VM recipes: `prepare`, `prepare-plan`, `rust`, `status`, `versions`, `build`,
`test`, `rock`, `models`, `postgres`, `cos`, `grafana-password`, `cos-endpoints`,
`cos-forward`, `metrics-server`, `metrics-server-local-tls`, `sandbox`,
`charm-pack [directory=charm]`, `charm-test`, `ratings-registry`,
`ratings-image [rock=""]`, `ratings-deploy`, `ratings-observability`,
`ratings-wait [seconds=600]`, `ratings-status`, `ratings-logs [lines=100]`,
`rollout`, `scale [units=3]`, `kubectl ...`, and `juju ...`.
`charm-init [directory=charm]` is an unrelated-project scaffold helper; do not run
it over the implemented Ratings charm.
Inside the VM use `just -f justfile.vm <recipe>`; bare `just` selects the host Justfile. Inspect recipes with host `just --list` or `just vm default`.

Restore discards post-snapshot VM state, including database data and credentials, but does not restore mounted sources. After `just restore prepared`, run `just kubeconfig` and check status.
Deletion permanently removes the selected VM's state/snapshots; verify `RATINGS_VM` first. Never use global `multipass purge` on a host with other VMs.

## Troubleshooting and references

- Mount failures: check `multipass info ratings-lab` (substitute your configured name), rerun `just create`, and inspect host Multipass mount support rather than silently switching providers. If mounts were disabled by local policy, review the security implications before enabling `multipass set local.privileged-mounts=true`.
- Build-provider failures: reconnect groups and inspect VM `lxc info` and `lxc profile show default`. Normal Linux-container craft builds do not need nested hardware virtualization; a deliberately selected VM build provider may.
- Export stalls: use the pipe-based `just kubeconfig` recipe, not direct file redirection from snapped `just` (see the AppArmor explanation above). The last progress message identifies VM inspection versus kubeconfig export. Try `timeout 30s multipass info ratings-lab` and `timeout 30s multipass exec --no-map-working-directory ratings-lab -- sudo -n k8s status --timeout=10s` (substitute your configured VM name). If sudo requires a password, inspect the VM's sudo configuration in `just shell` rather than removing `-n` from this unattended export.
- API failures: use `just info`, regenerate kubeconfig, and check VM IP/firewall/VPN access to TCP 6443. Do not disable API TLS verification.
- Pending pods: inspect events/storage/resources with `just kubectl get pods -A` and `just kubectl -n <namespace> describe pod <pod-name>` using actual names.
- Ratings waiting/blocked or a readiness timeout: inspect `just vm ratings-status`, `just vm ratings-logs 100`, and pod events in `ratings-dev`. Check the `ratings:database` relation, PostgreSQL readiness, charm configuration, and image-pull events before changing resources. A TCP-ready listener does not establish that later DB/Snap Store requests will succeed. Diagnose Juju hook errors with `just vm juju debug-log -m ratings-k8s:ratings-dev --replay --no-tail`; do not repeatedly rebuild the image to retry a failed hook.
- Artifact permission denied: `ratings-deploy` uses `/home/ubuntu/ratings-lab-artifacts/ratings_amd64.charm`, not the hidden `.cache` build output. Re-run `just vm charm-pack` to refresh that handoff, rather than weakening snap confinement.
- Image/registry failures: `ratings-image` expects one default rock or an explicit VM-side archive path. Inspect the VM's `ratings-registry.service` and registry/containerd configuration before changing exposure; the helper intentionally refuses unmanaged config and multi-node clusters. Do not bind the unauthenticated registry to `0.0.0.0` as a workaround.
- Logging integration failures: `ratings-observability` requires COS application `loki` in `ratings-k8s:cos-lite`. Inspect that model, the `ratings-logs` offer, and `ratings-loki` consumer alias; do not repoint unrelated offers. Either deploy COS intentionally or use the no-COS rollout sequence.
- gRPC failures: keep the host port-forward running and use `-plaintext`, the local proto, and a 64-character Authenticate `id`. Missing/expired Bearer metadata fails protected RPCs; reflection being unavailable is expected. Snap Store failures require checking workload egress and `snap-store-url`, not weakening Kubernetes API TLS.

Official references: [Ops environment tutorial](https://documentation.ubuntu.com/ops/latest/tutorial/from-zero-to-hero-write-your-first-kubernetes-charm/set-up-your-development-environment/), [Concierge configuration](https://canonical.com/juju/docs/concierge/reference/configuration/) ([v1.9.0 schema source](https://github.com/canonical/concierge/blob/v1.9.0/docs/reference/configuration.md)).
Also: [Canonical Kubernetes commands](https://documentation.ubuntu.com/canonical-kubernetes/latest/snap/reference/commands/), [Charmcraft pack](https://documentation.ubuntu.com/charmcraft/latest/reference/commands/pack/), and [Metrics Server v0.8.0 requirements](https://github.com/kubernetes-sigs/metrics-server/blob/v0.8.0/README.md).
