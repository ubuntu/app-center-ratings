# Contributing to Ratings

A big welcome and thank you for considering contributing to Ratings and Ubuntu! It’s people like you that make it a reality for users in our community.

Reading and following these guidelines will help us make the contribution process easy and effective for everyone involved. It also communicates that you agree to respect the time of the developers managing and developing this project. In return, we will reciprocate that respect by addressing your issue, assessing changes, and helping you finalize your pull requests.

These are mostly guidelines, not rules. Use your best judgment, and feel free to propose changes to this document in a pull request.

## Quicklinks

* [Code of Conduct](#code-of-conduct)
* [Getting Started](#getting-started)
* [Issues](#issues)
* [Pull Requests](#pull-requests)
* [Contributing to the code](#contributing-to-the-code)
* [Contributor License Agreement](#contributor-license-agreement)
* [Getting Help](#getting-help)

## Code of Conduct

We take our community seriously and hold ourselves and other contributors to high standards of communication. By participating and contributing to this project, you agree to uphold our [Code of Conduct](https://ubuntu.com/community/code-of-conduct).

## Getting Started

Contributions are made to this project via Issues and Pull Requests (PRs). A few general guidelines that cover both:

* Search for existing Issues and PRs on this repository before creating your own.
* We work hard to makes sure issues are handled in a timely manner but, depending on the impact, it could take a while to investigate the root cause. A friendly ping in the comment thread to the submitter or a contributor can help draw attention if your issue is blocking.
* If you've never contributed before, see [this Ubuntu discourse post](https://discourse.ubuntu.com/t/contribute/26) for resources and tips on how to get started.

### Issues

Issues should be used to report problems with the software, request a new feature, or to discuss potential changes before a PR is created. When you create a new Issue, a template will be loaded that will guide you through collecting and providing the information we need to investigate.

If you find an Issue that addresses the problem you're having, please add your own reproduction information to the existing issue rather than creating a new one. Adding a [reaction](https://github.blog/2016-03-10-add-reactions-to-pull-requests-issues-and-comments/) can also help be indicating to our maintainers that a particular problem is affecting more than just the reporter.

### Pull Requests

PRs are always welcome and can be a quick way to get your fix or improvement slated for the next release. In general, PRs should:

* Only fix/add the functionality in question **OR** address wide-spread whitespace/style issues, not both.
* Add unit or integration tests for fixed or changed functionality.
* Address a single concern in the least number of changed lines as possible.
* Be accompanied by a complete Pull Request template (loaded automatically when a PR is created).

For changes that address core functionality or would require breaking changes (e.g. a major release), it's best to open an Issue to discuss your proposal first. This is not required but can save time creating and reviewing changes.

In general, we follow the ["fork-and-pull" Git workflow](https://github.com/susam/gitpr)

1. Fork the repository to your own Github account
2. Clone the project to your machine
3. Create a branch locally with a succinct but descriptive name
4. Commit changes to the branch
5. Following any formatting and testing guidelines specific to this repo
6. Push changes to your fork
7. Open a PR in our repository and follow the PR template so that we can efficiently review the changes.

> PRs trigger charm unit and integration tests, Rust/Compose tests, linting and formatting checks, dependency-policy checks, and CLA/commit-style checks. All required checks must pass before merging into main.

Once merged to the main branch, `po` files and any documentation change will be automatically updated. Those are thus not necessary in the pull request itself to minimize diff review.

## Contributing to the code

### Repository layout

The [root README](README.md#repository-layout-and-guides) describes the monorepo.
The Python charm lives in root `src/`, `lib/`, and `tests/`, alongside
`charmcraft.yaml`, requirements locks, `pyproject.toml`, and `tox.ini`. The Rust
crate, its tests, protos, SQL, rock, Docker/Compose files, mock server, snap, and
`.env_files/` live under `ratings_rock/`. Root `justfile`, `justfile.vm`, and `dev/`
remain the orchestration entry points. The reusable [Terraform module](terraform/README.md)
lives in `terraform/`; environment-specific platform services, credentials, state,
and deployment inputs belong to the consuming operations stack, not the module.

### Required dependencies

* rust and cargo >= 1.69
* protobuf-compiler >= 3.21
* docker >= 24.04
* Docker Compose plugin >= 2.18 (`docker compose`)
* just

### Setup on Ubuntu

```bash
# Install the Rust toolchain
snap install rustup
rustup install stable

# Install build-time dependencies
sudo apt-get update
sudo apt-get install -y git gcc libssl-dev pkg-config protobuf-compiler

# Install the task runner
sudo snap install just --classic --channel=latest/stable

# Install Docker and the Compose plugin for integration tests (Ubuntu 24.04+)
sudo apt-get install -y docker.io docker-compose-v2
sudo usermod -aG docker $USER
newgrp docker

# Alternatively, if you wish to install docker using the script provided by
# Docker themselves:
curl -fsSL https://get.docker.com -o /tmp/get-docker.sh
sh /tmp/get-docker.sh
```

### Building and running the server

For local development, `ratings_rock/docker-compose.yml` runs the server alongside
a local Postgres database for manual testing and execution of the integration test
suite. Run these `justfile` recipes from the repository root; they select the
`ratings_rock/` working directory automatically. Compose explicitly retains the
`app-center-ratings` project name so relocating the files does not select new
project-scoped cache volumes:

```bash
# To build the release artifact (located in ./ratings_rock/target/release/ratings)
just build

# To start the local stack:
just compose-up

# To stop the local stack
just down

# To run only the unit tests
just test

# To run only the integration tests (requires the local stack to be up)
just integration-test

# To run all tests (requires the local stack to be up)
just test-all

# To start the stack, wait for readiness, run all tests, and stop it (also on failure)
just ci-test

# Pass a test filter and Cargo/test-harness arguments directly
just integration-test test_name -- --nocapture
```

`just ci-test` layers `ratings_rock/docker-compose.ci.yml` over
`ratings_rock/docker-compose.yml` and builds `ratings_rock/docker/ci/Dockerfile`
with `ratings_rock/` as the build context. This image installs the crate's `stable` Rust
toolchain and runs both the server and mock directly with `cargo run --locked`,
without `cargo-watch`. CI waits for PostgreSQL readiness before starting the
server, and for both the server and mock to be healthy before running tests.
Readiness has a 600-second limit (`just wait-for-server 900` selects a different
limit for a manual wait). Failures print container status and recent logs before
teardown; the GitHub Actions test step has a 30-minute overall timeout.
The override applies only to this recipe, including its cleanup; ordinary Compose
recipes retain the local development setup. It uses the same Compose stack, so
stop any local instance first.

`just up` still starts the Multipass VM, not Compose; use `just compose-up` or
`just up-detached` for the Compose stack. Workstation `just build` / `just test`
run locally; `just vm build` / `just vm test` retain their VM-specific behaviour.
Use `just --list` for all recipes, including the migrated Docker build, database
cleanup/shell, container shell, and volume-removal commands.

Manual Rust commands must run from the crate directory, not root `src/` (which
now contains the Python charm):

```sh
cd ratings_rock
cargo build --release
cargo test --lib
cargo fmt --all -- --check
# On a prepared Rockcraft/LXD build host, pack the workload here too:
rockcraft pack
```

The crate's `build.rs` reads `proto/`, SQLx embeds `sql/migrations/`, and Compose
uses the Docker files and `.env_files/` in this same directory. From the repository
root those paths are `ratings_rock/proto/`, `ratings_rock/sql/migrations/`, and
`ratings_rock/.env_files/`. The GitHub Actions Rust checks and rock pack action
also select `ratings_rock/`; root `just ci-test` handles the relocated Compose files.

### Charm development

Use Python 3.12 and run charm commands from the repository root:

```sh
python3 -m venv .venv
.venv/bin/python -m pip install -r requirements-dev.txt
.venv/bin/python -m pytest -q --cov=charm --cov-report=term-missing
.venv/bin/ruff check src tests
.venv/bin/ruff format --check src tests
# Optional, with tox installed:
tox -e lint,unit
# On a prepared Charmcraft/LXD build host:
charmcraft pack --platform amd64
```

The runtime and development requirements locks and `tox.ini` are at the root,
not under `charm/`. The preserved virtual environment moved from `charm/.venv`
to `.venv`; its activation scripts and executable shebangs may still point to the
old absolute path. If you use `source .venv/bin/activate`, rebuild it first with
`python3 -m venv --clear .venv` and reinstall `requirements-dev.txt`, then activate
the new environment. Do not rely on moving the environment to repair those paths.

Charm packing produces `app-center-ratings_amd64.charm` at the root. For the
prepared Multipass lab, root `just vm charm-test`, `just vm charm-pack`, and
`just vm rollout` use this checkout by default; the lab application is still
named `ratings`. No sibling Container Runner checkout is required.
`RATINGS_OPERATOR_REPO` remains an optional override for a compatible operator.
See the [operator details](README.md#ratings-kubernetes-sidecar-operator) and
[lab guide](doc/charming.md) for the full workflow and deployment safety boundaries.

### Charm and rock integration CI

[Integration tests](.github/workflows/integration_test.yaml) use
`canonical/charm-ci`, following MediaWiki's reusable workflow. The workflow
builds both artifacts declared in root `artifacts.yaml`; there is no separate
charm-only pack job. The rock is transferred as an Actions artifact and pushed
into the disposable MicroK8s cluster's local registry, **not GHCR or Charmhub**.
Root `concierge.yaml` provisions this CI environment; `dev/concierge.yaml`
remains the separate Multipass lab configuration.

`spread.yaml` discovers the live module under `tests/integration/`. Its smoke
check deploys the built charm and image with PostgreSQL, verifies readiness,
and exercises authentication, a ratings query, and user deletion over gRPC.
It does not access staging, DBaaS offers, or the Snap Store. This suite is
separate from the Rust integration tests and does not yet cover ingress,
observability, autoscaling, JWT rotation, or upgrades.

The integration dependencies are locked separately in
`requirements-integration.txt`. Safe validation without a cluster:

```sh
tox -e integration -- tests/integration --collect-only -q
tox -e integration -- tests/integration/checks.py -q
```

A live `tox -e integration` requires a **disposable** provisioned MicroK8s/Juju
environment and opcli's built-artifact manifest with the rock already pushed to
its local registry. Do not run it against a production controller. See
[tests/integration/README.md](tests/integration/README.md) for the exact contract,
cleanup behaviour, and lock-update command. Charmhub resource upload and channel
release remain separate from integration CI; see [publishing](doc/publishing.md).

### About the testsuite

The project includes a comprehensive testsuite made of unit and integration
tests. All the tests must pass before the review is considered. If you have
troubles with the testsuite, feel free to mention it on your PR description.

Rust unit tests are located within the files containing the code they are testing
under `ratings_rock/src/`. Offline Python charm tests live in root `tests/unit/`;
live charm/rock smoke tests live in root `tests/integration/`.

Rust integration tests are located in `ratings_rock/tests/` and run against the
local Docker Compose stack (see the [Building and running the server](#building-and-running-the-server)
section above for details).

The test suite must pass before merging the PR to our main branch. Any new
feature, change or fix must be covered by corresponding tests.

For more information on writing Rust tests, the following likes are useful:
  - <https://doc.rust-lang.org/book/ch11-01-writing-tests.html>
  - <https://doc.rust-lang.org/rust-by-example/testing/unit_testing.html>

### Code style

This project follow the [rust style-guide](https://doc.rust-lang.org/1.0.0/style/README.html).

## Contributor License Agreement

It is required to sign the [Contributor License Agreement](https://ubuntu.com/legal/contributors)
in order to contribute to this project.

An automated test is executed on PRs to check if it has been accepted.

This project is covered by [THIS LICENSE](LICENSE).

## Getting Help

Join us in the [Ubuntu Community](https://discourse.ubuntu.com/c/desktop/8) and
post your question there with a descriptive tag.
