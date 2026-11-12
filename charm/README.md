# The Ratings charm has moved

The Kubernetes sidecar charm is now maintained at the **repository root** of
[`canonical/container-runner-operator`](https://github.com/canonical/container-runner-operator),
in jj change `vpmyx` / bookmark `feat/ps7`. Its `main` branch retains the legacy
machine charm and must not be used for this Kubernetes deployment.

This directory is only a migration pointer; do not build or restore another copy
of the charm here. Existing ignored local environments/caches have been left
untouched. The standalone operator contains the source, vendored libraries,
dependency locks, tests, and build/contribution documentation.

For the application's Multipass lab, keep the operator checkout alongside this
repository or set host `RATINGS_OPERATOR_REPO` to its path. From the application
repository root:

```sh
just up
just operator-mount
just vm charm-test
```

`just create` mounts both checkouts for a new lab. `just vm charm-pack` and
`just vm rollout` use the standalone operator's VM-visible checkout. Tests alone
do not redeploy the workload; rollout does. See [the lab guide](../doc/charming.md)
for details.
