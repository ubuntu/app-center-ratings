# App Center Ratings

This monorepo contains the Ratings Rust service and workload rock, its Kubernetes
sidecar charm, a reusable Terraform module, and local development orchestration.

## Repository layout and guides

```text
app-center-ratings/
  charmcraft.yaml, src/, lib/, tests/       Python sidecar charm and offline tests
  requirements*.in, requirements*.txt      Charm dependency locks
  pyproject.toml, tox.ini                  Charm tooling
  ratings_rock/                           Rust service and workload packaging
    Cargo.toml, Cargo.lock, build.rs
    rust-toolchain.toml, deny.toml
    src/, tests/, proto/, sql/
    rockcraft.yaml, docker/, mock_server/, snap/, .env_files/
    docker-compose.yml, docker-compose.ci.yml
  terraform/                              Reusable Ratings application module
  justfile, justfile.vm, dev/              Workstation and VM orchestration
  doc/                                   Lab, publication, deployment, and migration guides
```

- [Rust service overview](ratings_rock/README.md) and [contributing](CONTRIBUTING.md).
- [Local Juju lab](doc/charming.md), [Charmhub publication](doc/publishing.md),
  [PS7 staging deployment](doc/deploy.md), and [database migrations](doc/database-migrations.md).
- [Reusable Terraform module](terraform/README.md), following the
  [MediaWiki charm module layout](https://github.com/canonical/mediawiki-k8s-operator/tree/main/terraform).
  It declares only the Ratings application and exposes its relation endpoints.
  Environment-specific model lookup, credentials, state, DBaaS/PgBouncer, ingress,
  and observability remain in the consuming
  [operations stack](https://github.com/canonical/ubuntu-engineering-terraform-models/tree/main/models/ps7/desktop/k8s-stg-desktop-managed-cluster/ratings),
  alongside the MediaWiki deployment reference; see the [staging plan](doc/plan.md).

Run `just build`, `just test`, and Compose recipes from this root; they select
`ratings_rock/` automatically. Manual Cargo and `rockcraft pack` commands require
`cd ratings_rock`; charm packing and Python requirements/tox commands run here.
`just vm rollout` uses this single checkout by default. `RATINGS_OPERATOR_REPO`
remains an optional compatible-operator override, not a required sibling repository.
The charm name and default archive are `app-center-ratings` and
`app-center-ratings_amd64.charm`; the local Juju application remains `ratings`.

## Ratings Kubernetes sidecar operator

> **Upgrade boundary:** The legacy Docker/Watchtower machine charm remains in
> [`canonical/container-runner-operator`](https://github.com/canonical/container-runner-operator/tree/main).
> This is not an in-place machine-to-Kubernetes upgrade; do not refresh an existing
> machine deployment with this repository's sidecar artifact.

The sidecar operator runs the [Ratings rock](ratings_rock/README.md)
(`/bin/ratings`, Pebble service `ratings`). All charm code, vendored libraries,
dependency locks, and tests are self-contained at this repository's root. The
charm does **not** deploy PostgreSQL, change the rock, deploy an ingress stack, or
install observability agents. It can optionally publish native gRPC routes to an
existing Traefik using `traefik_route`. It is specific to Ratings, not a generic
container runner.
The same charm artifact is intended for local development and Prodstacks 7 (PS7)
Juju deployments on Kubernetes; environment-specific configuration, database offers,
and OCI resources are supplied at deployment time. See the TLS and ingress
limitations below before exposing the service.

`charmcraft.yaml` is the sole source of charm metadata, configuration, and actions;
there are no separate `metadata.yaml`, `config.yaml`, or `actions.yaml` files.

## Requirements and packing

- Juju **3.6.4+**, including the lab's **3.6.29**. Check start/stop support requires
  3.6.4; the charm declares that minimum in `assumes`.
- Charmcraft **4.5.0**, Ubuntu **24.04**, Python **3.12**, Kubernetes sidecars.
- The declared build/deployment platform is **amd64** only. An arm64 rock and
  additional platform declaration would be needed for arm64.
- A compatible PostgreSQL `postgresql_client` provider, including a compatible
  PgBouncer provider in **session pooling** mode. The local lab uses PostgreSQL-k8s
  **16/stable** via its modern `database` endpoint; PS7 uses an approved DBaaS offer.
  The relation must supply the requested database, credentials and primary endpoint,
  plus CA data when database TLS is enabled. The user needs schema/migration
  permissions; transaction/statement pooling is not supported (see migrations below).
- For optional external gRPC ingress: a deployment-managed Traefik providing
  `traefik_route`, a TLS-capable `websecure` entrypoint and a trusted certificate
  covering `external-hostname`. Shared HAProxy, DNS, certificates, network policy
  and reachability are deployment responsibilities, not installed by this charm.

Build the charm from this repository's root, independently of the workload rock:

```sh
charmcraft pack --platform amd64
```

This produces **`app-center-ratings_amd64.charm`**. Repack after changing the charm.
Normal packing uses Charmcraft's isolated build environment; it does not deploy or
publish anything. CI packs on a disposable Ubuntu 24.04 runner using the
Charmcraft `4.x/stable` channel and `--destructive-mode`, without machine-charm
integration tests. Destructive mode can install build packages on that runner;
it is not a command to run on your everyday host.

The image is **not** included in the charm archive. Deployment must supply the
`ratings-image` OCI resource: the registry reference of the already-built rock
that is accessible to the Kubernetes nodes. Registry/import/rollout tooling is
outside the charm's scope and lives in the root orchestration recipes.

The Multipass lab and `just vm charm-pack`, `just vm charm-test`, and
`just vm rollout` recipes live at this repository's root. By default, `just create`
mounts this checkout once at `/home/ubuntu/app-center-ratings`; `just operator-mount`
validates the root charm without adding another mount. Only an explicit host
`RATINGS_OPERATOR_REPO` override selects and mounts a separate compatible operator
at `/home/ubuntu/ratings-operator`. No recipe switches branches. See the
[lab guide](doc/charming.md) for registry setup, dependency models, host forwarding,
and smoke checks. Rollout can publish images and deploy/refresh applications;
build/test recipes alone do not. Keep deployment artifacts outside hidden
folders because strictly confined Juju cannot read archives under `.cache`.

## Endpoints and configuration

| Endpoint | Role / interface | Purpose |
| --- | --- | --- |
| `database` | requires / `postgresql_client`, limit 1 | The configured SQL database (`ratings` by default) and relation-scoped credentials |
| `logging` | optional requires / `loki_push_api` | Native Pebble workload stdout/stderr forwarding to COS Loki |
| `traefik-route` | optional requires / `traefik_route`, limit 1 | Native gRPC Host routing on Traefik `websecure`, TLS termination then h2c to the Ratings Kubernetes Service |
| `ratings-peers` | peer / `ratings_peers` | Shared JWT secret reference and rotation notification |

After deployment by the rollout owner, the database relation is:

```sh
juju integrate -m ratings-k8s:ratings-dev \
  ratings:database postgresql:database
```

The lab's PostgreSQL application is named `postgresql`. No admin password,
extra roles, or superuser privileges are requested. In the lab, the provider
creates the database/user; the app owns its schema.

PS7 staging instead uses an existing **DBaaS-managed SQL database**,
`stg-app-center-rating`. Set `database-name` before relating the application and
use the approved staging provider/offer. The production SQL database is
`prod-app-center-rating`, recorded for inventory/export only; it is not the
staging connection target. SQL names are not Juju offer URLs or DBaaS service
identifiers. The managed provider must expose a compatible `postgresql_client`
relation and authorise this application to use that database; confirm credentials,
TLS, network access, and migration permissions with the DBaaS owners. This charm
does not provision a DBaaS instance or support arbitrary static connection secrets.
A compatible PgBouncer relation is also supported with session pooling; the charm
requests no provider-specific admin roles or direct connections.

For COS in the separate `cos-lite` model, offer the Loki provider's `logging`
endpoint from that model and integrate/consume that offer at **`ratings:logging`**
in `ratings-dev`. Use the actual Loki application and offer URL from that model.
No metrics or tracing endpoint is advertised: Ratings has no implemented metrics
or trace integration here. The Canonical log-forwarding library adds Juju model,
application, unit, and charm labels and disables targets when the relation goes
away. It uses Pebble's existing Loki support, not a Promtail binary or sidecar.
Private-CA Loki HTTPS is not configured by this charm; use an HTTP lab endpoint
or a certificate trusted by Pebble's container trust store.

Config options:

- `database-name`: defaults to `ratings` to preserve the local lab. Requests this
  SQL database through the relation, verifies the provider's returned name, and
  uses it in `APP_POSTGRES_URI`. Supports 1-63 ASCII letters, digits, underscores,
  or hyphens, with no leading hyphen. PS7 staging sets `stg-app-center-rating`.
  Set it at initial deployment, before creating the database relation. Changing
  it on an existing relation **blocks and stops the workload without retargeting
  the relation**; restore the previous value to recover. A database move needs a
  separately reviewed data/credential plan, not an ordinary config toggle. Do not
  remove/recreate a managed database relation without checking its data lifecycle.
- `external-hostname`: defaults to empty; used **only** when `traefik-route` is
  related. Examples: `ratings.localhost` (lab), `ratings.staging.ubuntu.com` (PS7).
  Only ASCII DNS labels of 1-63 letters/digits/hyphens, with alphanumeric ends,
  are accepted, up to 253 characters in total. Single-label names are allowed;
  uppercase ASCII is normalised to lowercase. URLs, ports, paths, wildcards,
  whitespace, empty labels, trailing dots, Unicode and Traefik rule syntax are
  rejected. An empty/invalid related hostname **blocks and replaces any previous
  route with empty HTTP routers/services**, without stopping an otherwise healthy
  backend. Correcting it republishes the route. With no relation, even an empty
  hostname leaves the existing direct/lab behaviour unchanged. Ingress-only config
  changes do not restart an unchanged Rust workload or rotate its JWT.
- `snap-store-url`: defaults to `https://api.snapcraft.io/v2/`; must be an absolute
  HTTP(S) URL ending in `/`, with no credentials, query, or fragment. Requests
  from the service go to that URL: the default requires network access to the
  public Snap Store.
- `rust-log`: defaults to `info,sqlx=warn,hyper=warn,reqwest=warn`, passed as
  `RUST_LOG`. Non-empty single-line Rust EnvFilter directives are supported.
  Arbitrary EnvFilter grammar is interpreted by Rust, not reimplemented in
  Python. Avoid verbose SQL/client debug filters: workload logs may then contain
  sensitive request data. Those logs are shared with COS when related.

The service receives:

```text
APP_HOST=0.0.0.0
APP_PORT=8080
APP_POSTGRES_URI=<relation-generated, percent-encoded SQLx URI>
APP_JWT_SECRET=<application-owned Juju secret>
APP_SNAPCRAFT_IO_URI=<snap-store-url>
RUST_LOG=<rust-log>
```

### Optional native gRPC ingress

The leader publishes a raw Traefik dynamic HTTP configuration through the official
`TraefikRouteRequirer(raw=True)` library. There is one model-UUID/application-unique
router and service, with exactly:

- ``rule: Host(`external-hostname`)``, `entryPoints: [websecure]`, `tls: {}`;
- `loadBalancer.servers: [{url: "h2c://<juju-application-name>:8080"}]`.

There is no `PathPrefix`, `StripPrefix`, path rewriting, HTTP entrypoint, HTTP
fallback or TLS certificate resolver in the route. Native gRPC method paths are
preserved. The backend target is the **Kubernetes Service**, not individual pod
addresses: this is Kubernetes service-based balancing, **not per-RPC balancing**.
Long-lived HTTP/2 connections can stay on one backend. This does not promise
Kubernetes readiness gating, end-to-end health checks, availability or HA. The
Pebble TCP check is only the charm's local listener readiness signal; transient
backend/check failures do not cause unnecessary route withdrawal.

The deployment owns the ingress stack. A shared-stack layout for **both** lab and
PS7 is:

```text
native gRPC client
  -> HAProxy edge (terminates client TLS, ALPN h2)
  -> model-local Traefik websecure (new HTTPS/HTTP2 connection from HAProxy)
  -> Ratings Kubernetes Service:8080 (plaintext HTTP/2, h2c)
```

`ingress-configurator` relates to Traefik's `upstream-ingress` and the edge's
`haproxy-route`; it configures routing, not another proxy hop. Configure its
`backend-protocol=https`, `paths=/`, and `allow-http=false`. The edge terminates
client TLS and establishes a separate TLS connection to Traefik, negotiating
HTTP/2 on both hops; this is **not TCP/SNI passthrough**. Traefik's server CA must
be transferred to the edge for backend trust. The pinned HAProxy 551 verifies the
backend CA but does not render backend SNI or hostname verification; account for
that upstream limitation in the trust boundary.

PS7 reuses the Desktop edge; the lab deploys a separate HAProxy machine charm in
LXD with private lab certificates. DNS/host forwarding must reach HAProxy, clients
must trust its public-facing certificate, and Traefik must resolve and reach
`h2c://<ratings-app-name>:8080`. This short Service name normally requires
Traefik and Ratings in the **same Kubernetes model/namespace**. A relation alone
cannot provide cross-namespace/cluster DNS, routing or network-policy access.

Once the deployment owner has supplied the stack, the **lab** relation/config
example (Ratings app `ratings`, existing Traefik app `traefik` in `ratings-dev`) is:

```sh
juju config -m ratings-k8s:ratings-dev ratings external-hostname=ratings.localhost
juju integrate -m ratings-k8s:ratings-dev ratings:traefik-route traefik:traefik-route
```

For **PS7 staging**, replace `PS7_CONTROLLER:STAGING_MODEL` with the approved
controller/model and `traefik` with the deployment's existing, reachable provider
in the same model/namespace. Keep the HAProxy host route, DNS, edge certificate,
and Traefik certificate aligned with `ratings.staging.ubuntu.com`. The operations
Terraform owns these settings and relations; the commands below illustrate the
contract, not an alternative to its reviewed apply workflow:

```sh
juju config -m PS7_CONTROLLER:STAGING_MODEL ratings \
  external-hostname=ratings.staging.ubuntu.com
juju integrate -m PS7_CONTROLLER:STAGING_MODEL ratings:traefik-route traefik:traefik-route
```

These are configuration examples, **not live-validated ingress deployments**.
The charm does not deploy HAProxy/Traefik, allocate addresses, publish DNS, issue
certificates or configure client trust. The empty `tls` map enables TLS termination
on Traefik; certificate provisioning/default-certificate selection is external.
Removing the optional ingress relation does not stop a healthy Ratings backend,
and the charm does not write to the dying relation. Relation creation/join/change,
config changes, leadership and charm upgrades republish the current configuration.
Only the leader writes application routing data; followers still validate config.

### TLS scope

The Ratings **gRPC listener remains plaintext in all environments**, including
PS7. `APP_TLS_KEY_PATH` and `APP_TLS_KEYCHAIN_PATH` are left unset. Port **8080/TCP**
is opened using `unit.set_ports(8080)`; that alone does not create external ingress
or imply TLS. Optional ingress protects the client-to-Traefik path as deployed
above, but the **final Traefik-to-Ratings hop is plaintext h2c**, not end-to-end TLS.
Keep it on a trusted/restricted network; do not expose port 8080 directly on an
untrusted network. Workload listener TLS, certificate relations, and access policy
are not implemented here.

Database TLS is handled separately. With `tls=False` (or no TLS flag), the SQLx
URI uses `sslmode=disable`, matching the lab's default. With `tls=True`, the charm
requires the provider's `tls-ca`, writes it to
`/etc/ratings/postgresql-ca.crt`, and uses `sslmode=verify-full` plus
`sslrootcert`. The provider endpoint hostname must match the certificate. Missing
CA data blocks the service rather than silently disabling verification. CA
rotation restarts the app even when the URI is unchanged. Credentials and TLS
fields are read through Canonical `data_interfaces`, including Juju secrets
where negotiated; passwords/usernames are percent-encoded independently.

### JWT ownership and rotation

The leader generates 48 cryptographically random bytes and encodes them with
**standard Base64** (`base64.b64encode`, not URL-safe Base64), matching the
`from_base64_secret` decoders in the application's
[`ratings_rock/src/jwt.rs`](ratings_rock/src/jwt.rs). The signing key is stored once
in an **application-owned Juju secret**, labelled `ratings-jwt`. Rotation uses the same
standard Base64 encoding. Only its ID and a non-secret generation marker are
published in the peer application databag.
All units of the application can access the same secret; no per-unit keys and no
plaintext peer credentials are used. Leader changes, charm upgrades, and ordinary
configuration hooks reuse it. The leader also recovers the existing labelled
secret if publication was interrupted. A missing *published* secret is never
silently regenerated.

Run the action on the **current leader** to rotate deliberately:

```sh
juju run -m ratings-k8s:ratings-dev ratings/0 rotate-jwt-secret
```

Replace `ratings/0` with the leader if necessary. The action returns only
`rotated: true`, not the secret. It updates the existing secret and peer generation
marker; the leader reconciles immediately and other units restart as peer changes
propagate. Application secret owners do not receive Juju `secret-changed`, so
peer notification is essential. Units use Ops `Secret.peek_content()` to read the
latest revision without the owner-invalid `refresh=True`. Relevant external
`secret-changed` events and database secret rotations are also handled.

**Rotation invalidates existing JWTs**, and units may temporarily use different
revisions while hooks propagate. There is no dual-key grace period. Accidental
secret deletion stops the workload; restoring secret availability or an explicit
operator-directed recovery is required. Database/JWT values necessarily enter the
workload's Pebble environment, which must be treated as sensitive. Do not share
unredacted `pebble plan`, environment dumps, or debug logs. The charm neither logs
credentials nor copies them into stored state or relation databags.

## Lifecycle and migrations

Before database credentials and the shared JWT are available, the charm disables
and stops even the image's default startup-enabled service. Invalid workload
configuration blocks and stops it; invalid optional ingress configuration blocks
and withdraws the route without stopping a healthy backend. Pending database data,
unavailable secrets, container connectivity, or readiness result in Waiting.
Database relation removal stops Ratings, disables its check,
erases the stale service environment/CA file, and closes the port. If Pebble is
unreachable at removal, the port is closed immediately and stopping is retried
once it is reachable. Database restoration reconfigures and starts the service.

A `ready`-level **TCP Pebble check** probes `127.0.0.1:8080` every five seconds,
with a two-second timeout and failure threshold one. A unit becomes Active only
when Pebble reports the service running and its check up. If an UP check has
zero successes or no success count, the charm also confirms the listener with a
bounded two-second TCP connection from the same pod before setting Active. A
failed or timed-out connection keeps the unit Waiting; missing, DOWN, or inactive
checks are never accepted as ready. This permits Active during the initial
post-replan health assessment without waiting for `update-status`: the first
success of an initially UP check does not emit a check-recovered event. There are
no sleeps or probe retry loops. Checks are reset when a service is restarted so
successes from the previous process cannot establish readiness for the new one.
Check-failed/recovered events and `update-status` update health; a check-failed
event reconciles without restarting an unchanged, still-running service. Pebble
restarts crashed services with backoff. Unchanged configuration does not restart
Ratings.
This is listener readiness, **not** a continuous DB transaction or end-to-end
Snap Store health test. DB/Snap Store outages after startup can affect requests
without bringing the TCP check down.

In the existing application source,
[`ratings_rock/src/main.rs`](ratings_rock/src/main.rs)
calls `check_db_conn()` before `grpc::run_server()`, and
[`ratings_rock/src/db/mod.rs`](ratings_rock/src/db/mod.rs)
executes
`sqlx::migrate!("sql/migrations").run(&pool)` while initialising the pool. That path
is relative to the Rust crate in `ratings_rock/`; the SQL files live in
[`ratings_rock/sql/migrations/`](ratings_rock/sql/migrations/). The rock
therefore embeds migrations and runs them before opening the listener. In the
locked **SQLx 0.8.6**, `sqlx-core/src/migrate/migrator.rs` enables locking by default
and calls `conn.lock()`/`unlock()`; `sqlx-postgres/src/migrate.rs` uses
`pg_advisory_lock`/`pg_advisory_unlock` with a database-derived lock ID. Concurrent
units serialize migrations through PostgreSQL itself. **No explicit charm
migration leader, migration Job, or extra PostgreSQL Python driver is necessary.**
PgBouncer must use **session pooling** so these session-scoped advisory locks and
SQLx's prepared statements retain the same PostgreSQL session. Transaction or
statement pooling is not compatible with this startup/migration model.

The application source removes an obsolete database-name-specific `CONNECT` grant
from its initial migration. Rebuild the rock to include the correction; repacking
only this charm does not change embedded SQL. Existing labs or restored databases
may need explicitly approved reconciliation of the known initial checksum; see
the [database migration runbook](doc/database-migrations.md).
The charm does not disable SQLx checksum checks or repair migration records.

## Dependencies, provenance, and tests

Unmodified, tracked Canonical libraries are pinned by commit, `LIBID`, API/patch,
and SHA-256 in `libraries.lock.json`:

- `charms.data_platform_libs.v0.data_interfaces`, **API 0 / patch 62**, commit
  `87e246a5392e9d3dd5c084a824a83c4bc1c34b77`; `PYDEPS`: `ops>=2.0.0`.
- `charms.loki_k8s.v1.loki_push_api`, **API 1 / patch 35**, commit
  `dbc2e7d78fcde4be11ae5328e7c882904befadf3`; `PYDEPS`: `cosl`.
- `charms.traefik_k8s.v0.traefik_route`, **API 0 / patch 7**, commit
  `99734ac72fda56b8d6138e411af343bc76c1fb9f`, `LIBID`
  `f0d93d2bdf354b99a527463a9c49fce3`, SHA-256
  `c727ee26196f9604a7d77cadbaa7bd26d8dfd82d246dc7310e0257e838704c09`.
  This pinned upstream file has **no `PYDEPS` declaration**; its `ops` and `yaml`
  imports are already supplied by the existing locked dependencies.

These pinned versions have no other mandatory charm-library imports. The Loki
library imports `cosl` for topology/rules helpers (the library's older prose about
fetching `juju_topology` does not match these current imports). Optional
`cryptography`/`psycopg` code paths in `data_interfaces` are not needed for this
same-model PostgreSQL consumer; it performs no direct database operations.
The sidecar code and tests originated in `ubuntu/app-center-ratings/charm`, were
previously developed in `canonical/container-runner-operator`'s `feat/ps7` sidecar
change, and now live at this repository's root under the **GPL-3.0**
[`LICENSE`](LICENSE). The layout move does not relicense that code as Apache.
Apache-2.0 licensing and copyright notices for vendored Canonical libraries remain
in their headers; the Traefik library is vendored unmodified. The legacy machine
implementation remains in `canonical/container-runner-operator` under its original
license. The repository's Canonical CLA policy still applies; see
[CONTRIBUTING.md](CONTRIBUTING.md).

`requirements.in` pins **Ops 2.23.2** and **cosl 1.11.3**. `requirements.txt` pins
all 14 runtime packages, including cosl's pydantic, diskcache, tenacity, PyYAML,
and typing dependencies and Ops' websocket/import metadata/OpenTelemetry API
dependencies. The latter is an Ops dependency, not a tracing integration.
Charmcraft uses strict dependency resolution; `pydantic-core` is binary-installed.
Development tools and their closure are separately locked in
`requirements-dev.txt`.

From this repository's root (Python 3.12 is the build/CI baseline):

```sh
python3 -m venv .venv
.venv/bin/python -m pip install -r requirements-dev.txt
.venv/bin/python -m pytest -q --cov=charm --cov-report=term-missing
.venv/bin/ruff check src tests
.venv/bin/ruff format --check src tests
.venv/bin/python -m pip check
```

The preserved `.venv` has moved from `charm/.venv` to the repository root.
Virtual environments are not relocatable: activation scripts and executable
shebangs may still contain the old absolute path. If using `source .venv/bin/activate`,
recreate the environment first (for example, `python3 -m venv --clear .venv`, then
reinstall `requirements-dev.txt`); moving it alone does not repair those paths.
Run optional `tox -e lint,unit` from the root, using the root `tox.ini` and locks.

The offline suite targets **99% charm-code coverage**. It includes immutable
database-name guards before the relation library's eager writes,
standard-Base64 JWT creation/rotation, zero/unknown
success counts with successful, refused, and timed-out TCP probes, non-UP checks,
check events without service restarts, and readiness reset after restart.
The [charm build workflow](.github/workflows/charm-build.yaml) runs these offline
tests, Ruff lint/format checks, and packs the root charm; its artifact upload path
is `app-center-ratings_amd64.charm`. That workflow does not deploy or publish to
Charmhub. The separate [Rust CI workflow](.github/workflows/ci.yaml) runs Compose
workload tests from `ratings_rock/` and has an explicitly gated rock-publication
job; see [publishing](doc/publishing.md). Optional `tox` commands use the same locked
dependencies and `pyproject.toml` configuration; see [CONTRIBUTING.md](CONTRIBUTING.md).

The Harness tests run **offline after dependency installation**, using the real
vendored libraries and fake Juju/Pebble/database data. They cover provisioning,
secret-backed and plaintext credentials, URI encoding, TLS/CA rotation, stable
leader/follower JWT ownership and rotation, invalid configuration, readiness,
ports, stop/recovery, disconnected containers, safe actionable failures, and
real Loki target/label/removal behaviour. Real-library ingress tests cover exact
raw TLS/h2c routing, hostname validation/normalisation and stale-route withdrawal,
app/model naming, optional/no-relation behaviour, leader/follower writes, config
changes without workload restart, relation recreation, leadership and upgrades.
The vendored checksums are also tested. These tests do not establish live gRPC,
certificate, readiness or HA behaviour.
Upstream `JujuVersion.from_environ()` deprecation warnings are expected with this
Ops version; vendored libraries are not modified to suppress them.
