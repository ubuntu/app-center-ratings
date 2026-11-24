# Real charm integration smoke test

`test_smoke.py` deploys the **built** `app-center-ratings` charm as `ratings`
with opcli's **built** `ratings-image` resource, and a dedicated single-unit
`postgresql-k8s` from `16/stable` (`profile=testing`, trusted for Kubernetes).
The `ratings:database` relation provisions credentials and the workload runs
its embedded migrations. Both applications and their agents must become
active/idle within 900 seconds; any Juju error fails the wait early.

The test generates Python gRPC clients from `ratings_rock/proto/*.proto` in a
temporary directory. It authenticates a random SHA-256 identity, checks that
`GetBulkRatings` returns no ratings for that unused snap ID, and deletes the
identity in `finally`. No votes, known snap IDs, Snap Store calls, offers,
DBaaS, staging services, external ingress, or supplied secrets are needed.
JWT rotation, TLS/ingress, scaling, and upgrades are outside this smoke suite.

## Runner contract

- Python 3.12 and `requirements-integration.txt` installed in an isolated tox
  environment. Bare `opcli` supplies its pytest plugin; `pytest-jubilant` is
  not needed because this suite owns the temporary-model fixture.
- A provisioned local MicroK8s cloud named `microk8s`, an available Juju
  controller for it, Juju >= 3.6.4, and usable `microk8s kubectl` on `PATH`.
  The intended CI host is Ubuntu 24.04 with MicroK8s
  `1.32-strict/stable` and Juju `3.6/stable`. MicroK8s needs DNS, storage,
  RBAC, and the local registry enabled; the test user must have access.
- `build/artifacts.build.yaml` must describe a single locally available charm
  and the built rock pushed to the node-accessible local registry by
  `opcli artifacts push-images --missing-registry deploy`. CI uses
  `upload-image: artifact`; no GHCR credentials are required. Both the workflow
  and the test plugin pin charm-ci/opcli to
  `14b1cf49d48b9d5b96192d9bca2890c98b7295cf` (v1.0.2).
- Tox invokes **`python -m pytest -v --tb=short {posargs:tests/integration}`**
  from the repository root. With no posargs it selects the integration directory,
  not `pyproject.toml`'s default unit tests. Spread discovers only `test_smoke.py`
  and opcli forwards `tox -e integration -- tests/integration/test_smoke.py`;
  that module **replaces** the default directory, avoiding duplicate suite runs.
  Do not put a literal `tests/integration` before `{posargs:tests/integration}`.
  Preserve `HOME`, `JUJU_DATA`, and runner Juju/MicroK8s access, and pass
  `OPCLI_ARTIFACTS_BUILD_YAML` through tox if overriding discovery. For manual
  flag-only selection, include the path: `tox -e integration -- tests/integration
  -k test_smoke` (otherwise posargs replace the integration default with flags
  and pytest falls back to unit-test discovery).

Artifacts are resolved lazily through the opcli `charm_path` and
`resource_images` fixtures. `--collect-only` does not build artifacts, generate
clients, create a model, or invoke Juju/kubectl. The plugin also supports
explicit `--charm-file app-center-ratings=/absolute/path/to/built.charm` and
`--resource-image ratings-image=localhost:32000/your-built-image:tag` inputs.
There is no fallback to the pre-existing root charm or a published image.
Pinned opcli can resolve an unpushed rock to its local `.rock` file; this suite
rejects that path before creating a model. The registry push must populate the
rock build's `image` field, which the plugin prefers over its retained `file`.
Skopeo is sufficient for this OCI-archive-to-registry push; Docker is not needed.

Every live test creates a fresh `jubilant-*` model on MicroK8s without changing
the CLI's current model or any controller/model defaults. Jubilant destroys
that model **and its storage** even when the test fails (cleanup has its own
600-second deadline). Existing-model/keep-model flags are deliberately not
supported. Never run this on a production controller.

Host-side `microk8s kubectl` forwards `service/ratings:8080` in the temporary
model's namespace to a random **127.0.0.1-only** port. Startup is limited to 45
seconds, channel readiness to 30 seconds, and each RPC to 10 seconds. The child
process is terminated/reaped on all exits, with kill escalation after 5 seconds.
The workload's plaintext gRPC transport is only accessed through that local
forward. The suite does not globally disable SSL verification; opcli separately
uses `--dest-tls-verify=false` when pushing to the local HTTP registry.
On setup/call failure, **these fixtures** print bounded, allowlisted Juju/pod
state summaries rather than raw logs, pod specs, relation data, credentials,
or tokens. RPC failures expose status codes only. This is not a guarantee about
all CI output: the pinned upstream reusable workflow also runs `juju status`
(with relations/storage) for available models and `juju debug-log --replay` on
failure. Jubilant wait errors can also include workload status messages.
The temporary model may already be gone by the workflow's diagnostics step;
review upstream log collection separately before introducing sensitive test data.

## Non-live validation and lock updates

From the repository root, in the integration dependency environment:

```sh
python -m pytest tests/integration --collect-only -q
python -m pytest tests/integration/checks.py -q
```

`checks.py` tests protobuf generation, mock deployment/RPC/process cleanup,
and the root artifact mapping, module-selection, and workflow pin/artifact
contracts using the installed pinned plugin. Registry push and MicroK8s
selection checks mock all external commands. Nothing invokes real Juju,
kubectl, containers, builds, or the Snap Store; checks do not fetch upstream
code. They require the committed root manifests/workflow to be present.
The filename intentionally avoids opcli/spread's `test_*.py` live discovery.
Keep virtualenvs outside `tests/integration`: pinned opcli recursively scans
that directory and does not respect pytest's virtualenv exclusions.

Regenerate only the integration lock (shared package versions are constrained
by the existing dev lock, but dev dependencies are not installed):

```sh
uv pip compile requirements-integration.in --python-version 3.12 \
  --output-file requirements-integration.txt --no-emit-index-url
```
