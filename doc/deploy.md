# Deploy Ratings to PS7 staging

The deployment uses **Charmhub for both the operator and its OCI workload
resource**. No separate GHCR publication or registry pull secret is required.

## 1. Build and upload the charm

Use this repository's Kubernetes sidecar charm in [`../charm/`](../charm/).
From that directory, with a prepared Charmcraft/LXD environment:

```sh
charmcraft pack
charmcraft login
charmcraft upload ./app-center-ratings_amd64.charm
```

Record the returned charm revision. Keep `charmcraft upload` in the directory
containing `charmcraft.yaml`; Charmcraft 4.5 can report `Project not configured
yet` after a successful upload from outside a project. Check
`charmcraft revisions app-center-ratings` before repeating an approved upload.

## 2. Publish the workload and release a compatible pair

The `.charm` archive contains the operator, not the Rust service image. Build the
rock from this repository's `rockcraft.yaml` and upload it as the charm's
`ratings-image` OCI resource, either with CI or manually using Charmcraft 4.5+:

```sh
charmcraft upload-resource app-center-ratings ratings-image \
  --image /path/to/ratings_0.0.4_amd64.rock
```

Record the returned **resource revision**, then release it with the selected
charm revision to the approved staging channel. See
[`publishing.md`](publishing.md) for the full CI-secret setup, build/upload,
release commands, and the Terraform provider's revision-zero limitation.

Uploading a resource or releasing a channel does not deploy it.

## 3. Review and apply the staging pins

Populate the operations repository's approved staging inputs:

- `ratings_charm_name` = `app-center-ratings`
- `ratings_charm_channel` = approved `track/risk`
- `ratings_charm_revision` = verified operator revision
- `ratings_image_revision` = verified positive `ratings-image` resource revision
- `staging_database_offer_url` = approved compatible staging DBaaS offer

Remove the obsolete `ratings_image` registry-reference input from local variable
files and deployment variable sets. The existing model and Terraform state
backend must already be provisioned. Follow the README in:

```text
ubuntu-engineering-terraform-models/models/ps7/desktop/k8s-stg-desktop-managed-cluster/ratings/
```

Plan, review, and apply through the operations process, then verify Juju status
and the native gRPC API. Do not use `just vm rollout` for PS7; it targets the local
lab. This staging deployment does not export or modify the legacy PS6 database.
