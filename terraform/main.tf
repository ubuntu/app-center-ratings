# Copyright 2026 Canonical Ltd.
# See LICENSE file for licensing details.

resource "juju_application" "ratings" {
  name        = var.app_name
  model_uuid  = var.model_uuid
  units       = var.units
  constraints = var.constraints

  charm {
    name     = var.charm_name
    channel  = var.channel
    revision = var.revision
    base     = "ubuntu@24.04"
  }

  resources = {
    "ratings-image" = tostring(var.image_revision)
  }

  config = var.config
}
