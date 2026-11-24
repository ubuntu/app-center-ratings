# Ratings Terraform module

This directory follows the [MediaWiki charm module layout](https://github.com/canonical/mediawiki-k8s-operator/tree/main/terraform).
It deploys **only the Ratings Kubernetes sidecar application** into an existing
Juju model. PostgreSQL/DBaaS, PgBouncer, ingress, COS collectors, model lookup,
provider credentials and remote state belong to the consuming operations stack.
It does not publish a charm or rock, deploy those platform services, or manage
Ratings' application-owned JWT secret.

## Structure

- `main.tf`: the `juju_application.ratings` resource.
- `variables.tf`: model UUID, application/charm name, explicit channel and
  independent charm/resource revisions, units, constraints and charm config.
- `outputs.tf`: `application` (the provider's application object) and `requires`
  (relation endpoint names, following the charm's `charmcraft.yaml` contract).
- `versions.tf`: Terraform >= 1.7 (including mock-provider tests) and Juju ~> 1.5.7.
- `tests/module.tftest.hcl`: plan-only tests using the mocked Juju provider.

## Use in a higher-level stack

```hcl
module "ratings" {
  source = "git::https://github.com/ubuntu/app-center-ratings//terraform?depth=1"

  model_uuid     = data.juju_model.service_model.uuid
  app_name       = "ratings"
  channel        = var.ratings_charm_channel
  revision       = var.ratings_charm_revision
  image_revision = var.ratings_image_revision
  units          = 1

  config = {
    database-name = var.database_name
  }
}

resource "juju_integration" "ratings_database" {
  model_uuid = data.juju_model.service_model.uuid

  application {
    name     = module.ratings.application.name
    endpoint = module.ratings.requires.database
  }

  application {
    name     = juju_application.pgbouncer.name
    endpoint = "database"
  }
}
```

The source uses the application's `ubuntu/app-center-ratings` origin, without an
invented release tag. It follows the neighboring operations MediaWiki Git source
style. **It will not resolve until this directory is merged to the remote default
branch.** A shallow source with no `ref` follows that branch; it is not a pin of
the Terraform module itself. Review module upgrades separately from charm and
image upgrades. Before merge, validate a temporary local-source fixture; do not
change the deployed source to a local path or initialize its backend.

### Inputs

| Input | Default | Purpose |
| --- | --- | --- |
| `model_uuid` | required | Existing Juju Kubernetes model UUID |
| `app_name` | `ratings` | Application name, independent of the charm name |
| `charm_name` | `app-center-ratings` | Name declared by the sidecar; override for a verified compatible published charm |
| `channel` | required | Approved `track/risk` or `track/risk/branch` |
| `revision` | required | Non-negative integer Charmhub charm revision |
| `image_revision` | required | Positive integer Charmhub `ratings-image` resource revision |
| `units` | `1` | Positive integer unit count |
| `constraints` | `arch=amd64` | Juju application constraints |
| `config` | `{}` | Charm configuration passed through unchanged |

The defaults for `charm_name`, base `ubuntu@24.04` and amd64 constraints match the
source charm/rock, **not a claim that a compatible revision is already published**.
No stable channel or deployment revisions are assumed. Select a tested pair;
charm and resource revisions have independent numbering. The module converts
`image_revision` to a string in `resources["ratings-image"]`. The pinned provider
rejects revision `0`; image URLs, digests, tags and local OCI archives are not
accepted here. Publication is a separate release workflow.

Supported charm configuration includes `database-name`, `snap-store-url`,
`rust-log` and `external-hostname`. Configuration defaults belong to the charm;
this module does not inject the staging SQL database or hostname. An empty
`config` leaves those defaults in effect (including database name `ratings`).
Use an explicitly approved database name in managed environments. Do not put
backend credentials or JWT keys in config; the database relation and charm-owned
Juju secret provide them. There is no expose block and no blanket Juju trust.

### Outputs and integrations

- `application`: the complete Juju application object, including `name`,
  `model_uuid` and `id` for consumers.
- `requires.database`: `database` (`postgresql_client`, at most one relation).
- `requires.logging`: `logging` (`loki_push_api`, optional).
- `requires.traefik_route`: `traefik-route` (`traefik_route`, optional, at most one
  relation). Configure `external-hostname` and use a compatible ingress charm
  for native gRPC; this is not a conventional path-prefixed ingress endpoint.

There are no `provides` outputs: the charm declares none. The `ratings-peers`
endpoint is internal, not an externally required integration.

## Existing Terraform state

When replacing a root application resource, the **consuming root** must retain:

```hcl
moved {
  from = juju_application.ratings
  to   = module.ratings.juju_application.ratings
}
```

Keep application name, model, charm/resource pins, config and constraints unchanged
during the address migration. The move itself is not a destroy/redeploy, but a
separately reviewed real plan is still required to confirm the existing state's
addresses and provider behavior. Do not remove the block until all affected
states have migrated; fresh installations have no old address to move.

Terraform `replace_triggered_by` requires a managed-resource address; a module
output cannot be used directly. The staging consumer tracks `application.id`
with a `terraform_data` resource and references its `output` attribute. This
adds a state-only identity bridge without adding workload resources or tying
relation replacement to every config/charm/resource update.

**First extraction plan with existing state:** for an address-only refactor, the
expected actions are an application address move with no application replacement,
creation of the bridge, and no relation replacements solely because of that
creation. `replace_triggered_by` reacts to updates or replacements of its trigger;
a create-only trigger is not eligible. Referencing `.output` also requires an
attribute change on an eligible action. The bridge's initially unknown output
therefore should not, by itself, replace existing relations. This is a review of
Terraform lifecycle semantics, **not an executed existing-state plan**.

**Bootstrap limitation:** the first plan has no previous application identity in
the bridge's state. If Ratings is also replaced in that plan, the bridge is still
being created rather than updated, so it does not itself propagate the application
replacement to existing relations. Keep the extraction separate from application
replacement and review the application and every existing relation's planned
actions before any approved apply. The plan-only mock suites start without
existing resource state and do not exercise this bootstrap case.

Once the bridge is recorded in state, changes to its planned output can trigger
relation replacements. It tracks ID/output changes, not every replacement action
of the encapsulated Juju resource: a same-name recreation with a known, unchanged
ID/output need not be propagated. Review replacement plans carefully; mocks
cannot verify live-controller identity or existing-state migration.

## Offline validation

With Terraform and an **existing** Juju provider cache available:

```sh
terraform init -backend=false -input=false -plugin-dir=/absolute/path/to/provider-cache
terraform fmt -check -recursive
terraform validate
terraform test
```

All module tests explicitly use `command = plan` with a mocked Juju provider.
They cover the endpoint contract, config pass-through, deployment settings,
independent pins and invalid numeric/channel/charm inputs. No test applies,
authenticates to Juju, publishes artifacts or checks a real service's readiness.

For pre-merge validation **without changing either source checkout's caches or
locks**, use the consuming ratings stack's `tests/validate_offline.py` helper.
It copies module/stack source into a disposable directory, substitutes a local
module source there only, initializes with `-backend=false` and the existing
provider cache, and runs formatting, validation and both mock plan suites. The
operations stack requires Terraform >= 1.10. Its backend and credentials are
never initialized, and its provider setup is excluded from its mock test harness.
