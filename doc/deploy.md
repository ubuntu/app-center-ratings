1. Build using your existing Multipass VM
From /home/berkus/Work/app-center-ratings:
just up
just operator-mount
just vm charm-test
just vm charm-pack
This builds the root-level charm from the sibling container-runner-operator checkout on vpmyx / feat/ps7, not the legacy machine charm on main.
The resulting archive is inside the VM:
/home/ubuntu/ratings-lab-artifacts/ratings_amd64.charm
Copy it to your host:
multipass transfer \
  ratings-lab:/home/ubuntu/ratings-lab-artifacts/ratings_amd64.charm \
  ./ratings_amd64.charm
These commands test and pack the charm; they do not deploy it. Don’t use just vm rollout for PS7—it targets the local lab.
2. Publish for the Terraform deployment
Our staging Terraform uses Charmhub, so it cannot deploy that local archive directly.
If publishing from the host, install the CLI:
sudo snap install charmcraft --classic --channel=4.x/stable
charmcraft login
Before uploading, confirm the registered charm name and publishing permissions. The current charmcraft.yaml names the charm ratings, regardless of the Git repository’s name. If the approved Charmhub name differs, update the metadata and rebuild first.
Once confirmed:
charmcraft upload ./ratings_amd64.charm
Record the returned revision and release it through the approved staging channel/resource publication process. Then populate the staging Terraform inputs:
ratings_charm_name
ratings_charm_channel
ratings_charm_revision
3. Supply the workload image separately
The .charm archive contains the operator, not the Rust service image. Staging also needs:
A Ratings rock published to a cluster-accessible registry.
Its digest-pinned reference in ratings_image.
The approved DBaaS staging offer in staging_database_offer_url.
The provisioned staging model and Terraform state backend.

~/Work/ubuntu-engineering-terraform-models/models/ps7/desktop/k8s-stg-desktop-managed-cluster/ratings/README.md
