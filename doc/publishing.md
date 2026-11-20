# Publish Ratings through Charmhub

We use **Charmhub only** for the operator and its workload image. There is no
separate GHCR push or GHCR pull credential to configure.

- [`../charm/charmcraft.yaml`](../charm/charmcraft.yaml) defines the
  `app-center-ratings` Kubernetes sidecar charm and its `ratings-image` OCI resource.
- [`../rockcraft.yaml`](../rockcraft.yaml) builds the Rust service, Pebble, and SQL
  migrations into an amd64 `.rock` OCI archive.
- Charmcraft uploads that image to Canonical's registry and registers a Charmhub
  resource revision. The `.charm` archive itself does not contain the workload image.

## 1. One-time publisher and CI setup

1. Have the publisher grant your account upload/release access to the registered
   `app-center-ratings` charm name. Do not register a different name as a workaround.
2. Install **Charmcraft 4.5 or newer** on the publishing machine:

   ```sh
   sudo snap install charmcraft --classic --channel=4.x/stable
   charmcraft version
   ```

   If already installed on that track, use `sudo snap refresh charmcraft --channel=4.x/stable`.
   Charmcraft 4.5 accepts a `.rock` path directly with `upload-resource --image`;
   no Docker import or separate Skopeo installation is needed for this upload.
   The command help still describes image IDs/digests, but the implementation also
   explicitly handles OCI archive paths.
3. Upload at least one compatible charm revision whose metadata declares
   `ratings-image` before uploading the resource (see the next section).
4. For GitHub Actions, export time-limited, charm-scoped upload credentials on a
   trusted workstation. This example expires after 30 days:

   ```sh
   umask 077
   charmcraft login --export=/tmp/ratings-charmhub.auth \
     --charm=app-center-ratings \
     --permission=package-manage-revisions \
     --permission=package-view-revisions \
     --ttl=2592000
   ```

   Add the file's **contents**, verbatim, as the repository Actions secret
   **`CHARMCRAFT_AUTH`**, using the approved secret-management process, then delete
   the temporary file. Restrict access to trusted publishing runs and rotate the
   secret before expiry. Never commit it, echo it into a build log, or include it
   in an artifact. This credential intentionally does not grant channel-release
   permission; releases remain a separate operator-approved step.

The account exporting the credentials must already have the required Charmhub
rights. GitHub's `GITHUB_TOKEN` cannot authenticate to Charmhub. Registry access is
handled by Charmcraft/Juju; the publisher and Desktop cluster still need network
access to Charmhub and its OCI/resource-delivery endpoints.

## 2. Build and upload the operator

From this repository's `charm/` directory, with a configured Charmcraft/LXD build
environment:

```sh
charmcraft pack
charmcraft login
charmcraft upload ./app-center-ratings_amd64.charm
```

Record the actual returned **charm revision**. If the intended compatible revision
is already uploaded, reuse it; an application-only image update does not require
a new charm build. Run `charmcraft upload` from the directory containing
`charmcraft.yaml` to avoid Charmcraft 4.5's post-upload `Project not configured yet`
error. If that error occurs after server approval, inspect the revisions before
retrying: the upload may already have succeeded.

```sh
charmcraft revisions app-center-ratings
```

## 3. Build and upload the workload image

### GitHub Actions

The `publish` job in [`../.github/workflows/ci.yaml`](../.github/workflows/ci.yaml):

1. Waits for the existing Rust build/lint/test job to pass.
2. Checks that `CHARMCRAFT_AUTH` is configured, then builds the amd64 rock with
   Rockcraft in LXD on Ubuntu 24.04.
3. Installs Charmcraft and runs:

   ```sh
   charmcraft upload-resource app-center-ratings ratings-image \
     --image <built-archive.rock> --format=json
   ```

4. Records the returned resource revision in the run summary, the job's
   `image-revision` output, and `ratings-image-revision.txt` in the
   `ratings-image-<full-commit-sha>` workflow artifact. The artifact also includes
   Charmcraft's `charmhub-resource.json` response. The run identifies the source commit.

Publication runs after a **push to `main`**, or manually from **Actions → CI →
Run workflow**, selecting **`main`**. Pull requests and manual runs on other
branches do not upload. The workflow must exist on the default branch for GitHub
to expose its manual-run button.

**CI uploads the resource only. It does not choose a charm revision, release a
channel, apply Terraform, or update running pods.** Rerunning the workflow can
create another resource revision; retain the actual returned revision for the
artifact you intend to deploy.

### Manual alternative

Build using the prepared lab from [`charming.md`](charming.md):

```sh
just rock
```

The archive is inside the VM under `/home/ubuntu/.cache/ratings/rock-src/`.
Enter the VM with `multipass shell ratings-lab` and run the following there with
Charmcraft 4.5+, or copy the archive to another configured publishing machine:

```sh
charmcraft login
charmcraft upload-resource app-center-ratings ratings-image \
  --image /home/ubuntu/.cache/ratings/rock-src/ratings_0.0.4_amd64.rock
```

Use the filename/version actually built. Alternatively, run `rockcraft pack` at
the application repository's root on an existing Rockcraft/LXD build host, then
pass the resulting `.rock` file to `--image`. Do **not** use `--filepath`: that
uploads a file resource rather than an OCI image resource.

Inspect resource revisions with:

```sh
charmcraft resource-revisions app-center-ratings ratings-image
```

## 4. Release an explicit charm/resource pair

Choose a compatible, tested pair and the approved staging channel. For example,
**if** the verified charm revision is `12`, the resource revision is `7`, and the
approved channel is `latest/edge`:

```sh
charmcraft release app-center-ratings \
  --revision=12 \
  --channel=latest/edge \
  --resource=ratings-image:7
charmcraft status app-center-ratings
```

These are example numbers, not deployment defaults. The channel now advertises
charm 12 with image resource 7. A channel is a movable release pointer, not a
model deployment. Charm and resource revisions are independent; neither is the
rock version, Git commit, image digest, or `.rock` file checksum.

## 5. Deploy the pinned pair through the operations repository

Set the reviewed staging inputs in:

```text
ubuntu-engineering-terraform-models/models/ps7/desktop/k8s-stg-desktop-managed-cluster/ratings/
```

For the example pair above:

```hcl
ratings_charm_name     = "app-center-ratings"
ratings_charm_channel  = "latest/edge"
ratings_charm_revision = 12
ratings_image_revision = 7
```

Remove the old `ratings_image` input from variable files, deployment variable
sets, and `TF_VAR_ratings_image`. Terraform now passes the resource revision as a
string in Juju's `resources` map, not an external image URL. Follow the operations
README's normal reviewed plan/apply procedure and retain the approved staging
DBaaS offer and other required inputs. Do not run lab rollout recipes against PS7.

**Provider limitation:** Juju Terraform provider 1.5.7 rejects resource revision
`0`, although Charmhub can assign it. The module therefore requires a positive
integer. If the intended image is only revision 0, upload it again and verify the
actual new resource revision before releasing/pinning it. Do not merely increment
the Terraform value or silently fall back to an external registry.

An image-only rollout changes `ratings_image_revision`, leaving the compatible
charm revision fixed. An operator-only rollout can retain the image resource pin.
Publishing or moving a channel does not override either explicit pin. Check Juju
status and the gRPC API after the separately approved apply; Terraform completion
is not an application-readiness test.

Keep the previous pair for rollback, but remember that selecting old artifacts
**does not undo database migrations**. Application or SQL migration changes need
a new rock; repacking only the charm is insufficient. Follow
[`database-migrations.md`](database-migrations.md) for existing database histories.

## References

- [Charmcraft upload-resource](https://documentation.ubuntu.com/charmcraft/stable/reference/commands/upload-resource/)
- [Charmcraft OCI archive support (UploadResourceCommand)](https://github.com/canonical/charmcraft/blob/main/charmcraft/application/commands/store.py)
- [Charmcraft remote authentication](https://documentation.ubuntu.com/charmcraft/stable/howto/manage-the-current-charmhub-user/)
- [Juju provider 1.5.7 application resources](https://registry.terraform.io/providers/juju/juju/1.5.7/docs/resources/application)
- [Rockcraft OCI registry publishing](https://ubuntu.com/containers/rockcraft/docs/1/how-to/crafting/publish-a-rock/)
