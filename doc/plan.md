# Ratings PS6 database export and PS7 staging deployment plan

## Context and decisions

The production history below was confirmed by the project maintainer. A relevant
operations repository has now been identified:
[`canonical/ubuntu-engineering-terraform-models`](https://github.com/canonical/ubuntu-engineering-terraform-models),
checked locally on 2026-11-11 (checkout HEAD `bd3f539`). Ratings staging
configuration now lives in
`models/ps7/desktop/k8s-stg-desktop-managed-cluster/ratings/` in that repository.
The PS6 production configuration and export access are still being located.

The maintainer identifies MediaWiki as the team's latest PS7 deployment and the
primary reference for Ratings. Use its Kubernetes deployment pattern as the PS7
design baseline, retaining the current Ratings sidecar charm. Earlier PS7
documentation was reported to describe machine-model-only support; clarify that
documentation's scope and arrange Ratings' model/access with platform owners,
rather than treating machine-only deployment as the default. The reference has
been inspected as configuration, not independently validated against live state.

The intended hosting platform is the **existing Desktop-owned Kubernetes cluster
reportedly hosting MediaWiki**, not a new cluster. The maintainer has identified
this as the Ratings **staging** target, but MediaWiki's actual cluster placement
still needs confirmation. Deploy Ratings alongside MediaWiki at cluster level,
with its own staging Juju model/namespace following the team's isolation
conventions; do not assume that sharing a cluster means sharing MediaWiki's model
or namespace.

**Current scope:**

- **PS6 production:** access the existing deployment only to identify and dump the
  database (Operation Shadowgrab). No redeployment, upgrade, traffic change, write
  freeze, or retirement is planned.
- **PS7 staging:** deploy Ratings with the existing sidecar charm on the Desktop
  cluster, using MediaWiki as the reference. An authorised copy of the PS6 dump
  can be used to validate restore and application migrations in staging.
- **Out of scope:** PS7 production deployment, production traffic cutover,
  continuous PS6-to-PS7 replication, and PS6 decommissioning. These require a
  separate future plan, not completion of this staging work.

- `canonical/app-center-ratings-operator` was renamed to
  [`canonical/container-runner-operator`](https://github.com/canonical/container-runner-operator)
  when the Ratings-specific operator was generalised. They are not separate
  deployment projects.
- The Ratings PostgreSQL connection described in that operator documents the
  database relation used by the legacy **prodstack 6 (PS6)** deployment.
- The existing PS6 deployment uses a machine charm. Machine models also appear
  in the PS7 operations repository; machine charms are not a PS6-only option.
- A Kubernetes sidecar charm cannot be deployed directly into a Juju machine
  model: it needs a Kubernetes cloud/model. Neither blanket Kubernetes support
  nor blanket machine-only availability can be inferred for Ratings from the
  evidence currently available.
- Keep the current Ratings sidecar charm in [`../charm/`](../charm/) and the
  local Kubernetes lab. The sidecar charm remains the preferred implementation
  if Ratings can use an approved PS7 Kubernetes model. The lab is not yet a
  production-ready deployment. Setup is in [charming.md](charming.md).
- Follow MediaWiki for the PS7 staging platform integrations and target the same
  Desktop-owned Kubernetes cluster. Confirm cluster identity, staging model
  provisioning, access, and operational ownership. Record any blocker without
  silently changing the target to a machine deployment or PS7 production.
- We still need controlled workload updates through the selected charm's supported
  Juju interfaces. The operations examples do not establish an existing automatic
  workload-image promotion mechanism.

There are two operational stories: a PS6 production database export and a PS7
staging deployment. They are linked by optional authorised restore testing, not
by a production migration or traffic switch. Staging can be deployed with an
empty/synthetic database while export approval is pending. Do not remove or
replace the working local lab while investigating PS6.

## Evidence from the PS7 operations examples

Paths in this section are relative to `ubuntu-engineering-terraform-models`.
The inspected files configure applications in existing models using
`data "juju_model"`; they do not provision those models or their backing clusters.
Model names alone are not proof of model type; the Kubernetes modules/charms below
provide the stronger evidence.

- **Primary reference: MediaWiki, the team's latest PS7 deployment**
  (maintainer-confirmed):
  `models/ps7/desktop/k8s-stg-desktop-managed-cluster/mediawiki/main.tf` declares
  MediaWiki and MySQL Router Kubernetes modules, Valkey, Traefik, and an ingress
  configurator connected to shared PS7 ingress. Its `cos.tf` deploys
  `opentelemetry-collector-k8s` and relates telemetry to PS7 aggregator offers.
  Use this deployment as the primary platform-integration reference, adapting it
  for Ratings rather than copying the application stack. Its HTTP ingress does
  not by itself establish gRPC/HTTP2 support for Ratings.
- **Additional Kubernetes workload example:**
  `models/ps7/ubuntu-engineering/k8s-prod-upki-mirror-ps7-default/main.tf` declares
  `upki-mirror-k8s`, pinned charm revisions, and an ingress-configurator integration
  with a shared PS7 ingress offer. The neighbouring
  `k8s-prod-ubuntu-manpages-ng-ps7-default/main.tf` shows a similar routing pattern.
- **Separate machine database:**
  `models/ps7/desktop/stg-desktop-mediawiki-db/main.tf` uses the MySQL machine module
  and publishes a database offer consumed by MediaWiki's Kubernetes configuration.
  The Kubernetes example comments out its MySQL Router/backend integration as
  problematic and instead declares a direct application/database-offer relation.
  This is not a validated PostgreSQL design: confirm Ratings' relation interface,
  networking, TLS, and support independently.
- **Machine application:**
  `models/ps7/desktop/stg-desktop-versions/main.tf` declares an application,
  ingress configurator, and Landscape subordinate. A fuller example,
  `models/ps7/ubuntu-engineering/prod-ubuntu-kpi-ps7/main.tf`, includes host-management
  modules, an egress proxy, and the shared-ingress offer integration.
- **Credentials, state, and revision control:** examples use Vault-backed Juju
  credentials, per-model remote Terraform state, and explicit charm revisions.
  Terraform module source refs and deployed charm revisions are separate controls;
  do not assume one pins or upgrades the other. Keep credentials and state out of
  this repository. Review any edge channels, unpinned module sources, and privileged
  deployments rather than copying the examples verbatim.
- **Automation boundary:** `README.md` explicitly says not to add the repository
  to repo automation for now. `renovate.json` describes dependency-update PR
  configuration, not workload-image deployment. The inspected GitHub workflows
  cover QA, Renovate validation, and scheduled database backups; they do not define
  Terraform apply or application promotion. No Ratings/Shadowgrab configuration
  was found in the searched model Terraform/Markdown/YAML files, and no Container
  Runner/image-update configuration was found in the scoped PS7 search. External
  deployment automation may still exist.

## Story 1: PS6 production database dump (Operation Shadowgrab)

**Goal:** As an operator, identify and access the existing PS6 production database
and safely obtain its dump for later restore/migration testing. PS6 is an export
source only; do not redeploy or modify its service, database schema, or traffic.
This document does not imply that an export has already been performed.

**PS6 source locator (maintainer-provided):**
`admin/prod-desktop-ratings-db.postgresql`

In Juju offer-URL syntax this identifies owner `admin`, model
`prod-desktop-ratings-db`, and offer `postgresql`. Confirm the backing PostgreSQL
application and relation through the approved PS6 controller. This locator is not
a PostgreSQL connection URI: it does not supply the SQL database name, host/port,
credentials, or controller identity. It identifies the production export source
only, never the PS7 staging application's database connection.

### 1. Discover the deployed system

- [ ] Locate the operations repository, deployment bundle/overlays, and runbooks.
- [ ] Resolve `admin/prod-desktop-ratings-db.postgresql` on the approved PS6
      controller; record the cloud/controller, backing application, and deployed
      database charm revision. Do not consume the production offer from staging
      merely to obtain access for a dump.
- [ ] Identify the PostgreSQL version, SQL database name, relation, primary endpoint,
      backup facilities, and approved access route; do not infer these from the
      offer name or the local lab.
- [ ] Identify operational owners and obtain approval for production access and
      data export. Use read-only inspection until an export procedure is agreed.

The renamed operator provides useful context, but its current defaults are not
proof of the configuration deployed on PS6. Do not guess production endpoints,
credentials, charm revisions, or database names from the local lab.

### 2. Prepare and rehearse the export

- [ ] Agree the data scope, destination, authorised recipients, retention period,
      and deletion policy, including whether production data may enter staging
      and what sanitisation is required. Treat the dump as sensitive production
      data; do not commit dumps or credentials or place them in CI artifacts.
- [ ] Choose a PostgreSQL-compatible logical dump/restore procedure and client
      versions after identifying the source and target versions. A custom-format
      `pg_dump` archive is a candidate, not an already agreed production command.
- [ ] Plan access through the approved operator path and credentials with the
      minimum required privileges. Avoid exposing PostgreSQL publicly.
- [ ] Assess source load, dump duration, and storage requirements. Use an existing
      backup or isolated restored copy for rehearsal where operationally suitable.
- [ ] Document ownership/role handling, required extensions, and a consistent dump
      scope. A logical dump gives a snapshot; it does not capture later writes.
- [ ] Encrypt the export and transfer through an approved channel; record a
      checksum, source/version, snapshot timing, and tool versions without secrets.

### 3. Verify and hand off the dump

- [ ] Verify archive readability, checksum, and completeness using the agreed scope;
      validate restoration into an isolated PostgreSQL instance where authorised.
- [ ] Record the snapshot timing and document that subsequent PS6 writes are not
      included. This export does not require a production write freeze or catch-up.
- [ ] Hand off the protected archive and restore notes through the approved path;
      apply the agreed retention/deletion policy to temporary copies.
- [ ] Leave the PS6 deployment unchanged. Do not upgrade its charm/image, run new
      application migrations against it, switch traffic, or decommission it.

**Acceptance:** the source database is identified, an authorised dump is obtained,
its integrity is verified, and access/restore notes are recorded without secrets.
PS6 production remains unchanged. No production cutover runbook or replacement
machine deployment is required for this story.

## Story 2: PS7 staging Kubernetes deployment using MediaWiki as the reference

**Goal:** As an operator, deploy Ratings to **PS7 staging** using the existing
Kubernetes sidecar charm on the same Desktop-owned cluster as MediaWiki. Use
MediaWiki as the platform reference for routing, observability, and operations.
Validate upgrades and, when authorised, restore/application migrations against a
copy of the PS6 dump. This is not a production replacement or traffic cutover.
Container Runner is relevant to understanding the legacy export source, not a
new deployment required by this story.

### 1. Establish the PS7 staging deployment contract

- [ ] Confirm MediaWiki's actual Desktop cluster and Juju cloud/controller;
      the reference model is `k8s-stg-desktop-managed-cluster-mediawiki`.
      Arrange Ratings' staging placement on the same cluster. PS7 production
      placement is outside this plan.
- [ ] Arrange Ratings' own model/namespace on that existing cluster using the
      team's conventions. Confirm access, tenancy, quotas, support, and cluster
      lifecycle owners; do not reuse MediaWiki's namespace by default. Record the
      older machine-model-only documentation's link/version and clarify its scope.
- [x] Place Ratings staging configuration in the operations repository at
      `models/ps7/desktop/k8s-stg-desktop-managed-cluster/ratings/`, alongside MediaWiki.
- [ ] Supply and verify that configuration's actual cloud/controller, model owner/
      name, platform versions, networking, and operational owners.
- [ ] Record the target: current Ratings sidecar charm in its own provisioned
      PS7 staging Kubernetes model/namespace; record any platform blocker explicitly.
- [ ] Define a staging PostgreSQL service/relation, storage, backup/restore ownership,
      availability requirements, and an isolated restore destination. The staging
      application must never use PS6 production database credentials or endpoints.
- [ ] Define staging gRPC routing/load balancing, TLS termination, DNS, access
      restrictions, and staging-only secrets/JWT signing keys. Do not copy production
      signing keys or redirect production clients to staging.
- [x] Keep deployment Terraform and its runbook in the operations repository's
      `ratings/` directory; keep this repository responsible for application/charm
      artifacts and their tests. The initial scaffold declares the application,
      a required staging database offer, and optional OTEL log forwarding; it has
      not been applied and does not implement external gRPC ingress.

### 2. Adapt the MediaWiki reference and validate Ratings

The deployment directory is
`models/ps7/desktop/k8s-stg-desktop-managed-cluster/ratings/` in the operations
repository. Its `README.md` documents the current scaffold, required artifact/model/
database/backend inputs, validation, and the remaining ingress work. The current
Terraform provider requires the sidecar charm to be published to Charmhub; the
lab's local `.charm` archive is not directly deployable through that provider.

The reference directory is
`models/ps7/desktop/k8s-stg-desktop-managed-cluster/mediawiki/` in the operations
repository. Reuse its platform conventions, not its application-specific services:

| Reference area | Ratings adaptation |
| --- | --- |
| `providers.tf`, `backend.tf`, `versions.tf` | Follow Vault/Juju authentication and remote-state conventions; use Ratings' own approved identity, state location, and provider constraints. |
| `main.tf`: existing model and application declarations | Target Ratings' own staging model/namespace on the same Desktop cluster as MediaWiki, deploy the existing Ratings charm, and pin charm and workload-image artifacts. |
| `main.tf`: Traefik, ingress configurator, shared ingress offers | Follow the routing topology; verify end-to-end gRPC/HTTP2, backend protocol, TLS termination, and health checks. Add compatible charm relations if required. |
| `cos.tf`: OpenTelemetry collector and aggregator offers | Follow the platform telemetry topology and certificate handling; verify Ratings' log integration and implement metrics/traces separately. |
| Separate `stg-desktop-mediawiki-db` model and database offer | Use the separation/ownership pattern where appropriate, but design and validate PostgreSQL relations, connectivity, TLS, backups, and Shadowgrab restore independently. |

Do not bring over MediaWiki, MySQL Router, Valkey, or media-upload/object-storage
configuration unless Ratings has a separately demonstrated requirement.

- [ ] Confirm that the chosen charm can run the current Ratings image with the
      required environment, database credentials, secrets, and port configuration.
- [ ] Review health checks, restart behaviour, shutdown/draining, startup migrations,
      resource sizing, availability during upgrades, and database connection limits.
- [ ] For Kubernetes, validate pod/network policies, requests/limits, storage, and
      charm permissions. Adapt platform ingress and telemetry relations from the
      examples only after verifying compatibility with this charm and gRPC.

- [ ] Integrate platform logging, monitoring, and alerting using supported charm
      interfaces. The lab currently forwards logs; application metrics/traces
      require implementation, and local Loki wiring is not the PS7 OTEL design.
- [ ] Define unit scaling, load balancing, and a single owner for scaling decisions.
      Do not let an HPA and Juju independently own Kubernetes replica counts.
- [ ] Deploy and smoke-test staging with empty/synthetic data independently of
      Shadowgrab. Once approved, restore a permitted/sanitised PS6 copy into an
      isolated staging database; verify schema, row counts, sequence state,
      constraints, startup migrations, and representative gRPC results. Do not
      point the staging application at the production source to perform these tests.

### 3. Design Juju-aware automatic updates

The inspected operations examples pin charm revisions and Terraform module refs
where specified; they do not demonstrate automatic workload-image promotion.
Respect the repository's automation warning and establish the approved apply
process before adding an updater. Kubernetes rolling updates alone do not detect
new registry images or define promotion policy.

Container Runner's Watchtower behaviour is legacy context only: do not change
PS6 update settings as part of this work. Design the new update path for the PS7
staging sidecar deployment.

A candidate is an operations CI/CD promotion workflow that:

1. Builds and tests the Ratings image and compatible charm; publishes immutable,
   traceable artifacts to the approved registries.
2. Records the selected image digest, charm revision, and configuration in a
   reviewed deployment change, with automatic promotion only where policy allows.
3. Applies staging changes through the approved Terraform/Juju workflow and the
   sidecar charm's workload-image resource interfaces. Validate digest support and
   avoid competing direct Juju-managed StatefulSet/pod updates. Reconcile all
   changes with Terraform's recorded desired state.
4. Waits for readiness, checks gRPC functionality and operational signals, and
   stops promotion on failure.
5. Retains the previous artifact references and a tested recovery procedure.
   Application rollback may not undo SQL migrations; define schema compatibility
   and database recovery separately.

- [ ] Choose the automation and promotion policy with the operations team using
      the identified repository. Validate the exact upgrade workflow against the
      chosen charm and deployed versions before documenting executable commands.
- [ ] Decide staging update cadence, test/approval gates, alerting, and recovery
      ownership. Production promotion is a separate future scope.

### 4. Keep platform provisioning separate from application deployment

Keep the current sidecar charm and local lab unchanged. The staging plan reuses
the existing Desktop cluster; it does not provision a replacement cluster.
Confirm its identity and environment with the Desktop owners, then arrange the
Ratings model/namespace, access, capacity, and operational ownership there. The
inspected application Terraform reads an existing model, so model provisioning
must be coordinated separately from deploying the Ratings applications.

Sharing the cluster does not imply sharing application namespaces, credentials,
Terraform state, or database instances. Confirm which ingress and observability
offers are shared services and which resources Ratings needs to own. Do not
bootstrap another cluster to bypass an access or support restriction.

**Acceptance:** Ratings is deployed and reachable through its staging gRPC route
on the confirmed Desktop cluster, with its own staging model/namespace, database,
and secrets. Security, observability, scaling ownership, and staging upgrade/
recovery procedures are tested; platform access/support/ownership are recorded.
Any authorised Shadowgrab restore is isolated from production and its migration
test results are documented. No production traffic or PS6 deployment changes
are required to complete the staging deployment.

## Immediate next steps

1. Populate the verified inputs for the operations repository's staging `ratings/`
   configuration; fill in the PS6/PS7 inventory without secrets. Do not apply until
   model access, staging database offer, artifact publication, and state are approved.
2. Agree and rehearse Operation Shadowgrab with the PS6 operational owners.
3. Confirm MediaWiki's Desktop cluster placement, arrange Ratings' staging model
   on that cluster, and adapt its integrations to the existing Ratings sidecar charm.
4. Agree the Terraform/Juju apply and promotion process; test the chosen deployment
   in staging without enabling unapproved repository automation.
5. Verify the PS6 dump and, where authorised, test restoring a protected/sanitised
   copy into isolated staging PostgreSQL. Do not plan a production cutover here.
