# Welcome to Ratings

[actions-image]: https://github.com/ubuntu/app-center-ratings/actions/workflows/push.yaml/badge.svg
[actions-url]: https://github.com/ubuntu/app-center-ratings/actions/workflows/push.yaml

[license-image]: https://img.shields.io/badge/License-GPL3.0-blue.svg

<!-- [codecov-image]: https://codecov.io/gh/ubuntu/app-center-ratings/branch/dev/graph/badge.svg -->
<!-- [codecov-url]: https://codecov.io/gh/ubuntu/app-center-ratings -->

[![Code quality][actions-image]][actions-url]
[![License][license-image]](LICENSE)
<!-- [![Code coverage][codecov-image]][codecov-url] -->

This is the code repository for **Ratings**, the backend service responsible
for managing the new vote and rating system used in [App Center](https://github.com/ubuntu/app-center)

For general details, including installation, getting started and setting up a
development environment, please see [CONTRIBUTING.md](CONTRIBUTING.md).

## Local Juju deployment

For a local Kubernetes and Juju deployment using Multipass, see
[doc/charming.md](doc/charming.md). With the lab already prepared and COS Lite
deployed, run from the repository root:

```sh
just up
just vm rollout
just vm ratings-status
just kubeconfig
just ratings
```

`rollout` builds the rock and charm, publishes a digest-pinned image to a
VM-loopback registry, deploys or refreshes Ratings, integrates PostgreSQL and COS
Loki logging, and waits for readiness with a 600-second readiness deadline.
`just ratings` stays in the foreground and forwards **plaintext gRPC** to host
**`127.0.0.1:8080`**; it is not a web page or public ingress. See the guide for the
[proto-based API smoke check](doc/charming.md#host-local-grpc-smoke-check),
[rollout without COS and subsequent refreshes](doc/charming.md#7-deploy-ratings-and-use-the-api),
and Juju-managed scaling. These commands do not assert that a deployment has
already passed runtime validation.

The implemented Ratings operator lives in [`charm/`](charm/); see its
[README](charm/README.md) for endpoints, configuration, JWT rotation, and charm
development.

## Dependencies

In order to run, this needs you to install at minimum `libssl-dev` (for
OpenSSL) and `protobuf-compiler` (for `prost`) via `apt`.

Please see [CONTRIBUTING.md](CONTRIBUTING.md) for details.

## Get involved

This is an [open source](LICENSE) project and we warmly welcome community
contributions, suggestions, and constructive feedback. If you're interested in
contributing, please take a look at our [Contribution guidelines](CONTRIBUTING.md) first.

- To report an issue, please file a bug report against our repository.
- For suggestions and constructive feedback, please file a feature request or a bug report.

## Get in touch

We're friendly! We have a community forum at [https://discourse.ubuntu.com](https://discourse.ubuntu.com)
where we discuss feature plans, development news, issues, updates and troubleshooting.

For news and updates, follow the [Ubuntu twitter account](https://twitter.com/ubuntu)
and on [Facebook](https://www.facebook.com/ubuntu).
