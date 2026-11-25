mock_provider "juju" {}

variables {
  model_uuid     = "00000000-0000-4000-8000-000000000001"
  channel        = "latest/edge"
  revision       = 1
  image_revision = 7
}

run "minimal_application_plan" {
  command = plan

  assert {
    condition = (
      output.application.name == "ratings" &&
      output.application.model_uuid == var.model_uuid &&
      output.application.units == 1 &&
      output.application.constraints == "arch=amd64" &&
      length(output.application.expose) == 0 &&
      !output.application.trust
    )
    error_message = "Deploy only an internal, untrusted, single-unit amd64 Ratings application in the supplied model."
  }

  assert {
    condition = (
      one(output.application.charm).name == "app-center-ratings" &&
      one(output.application.charm).channel == var.channel &&
      one(output.application.charm).revision == var.revision &&
      one(output.application.charm).base == "ubuntu@24.04" &&
      output.application.resources["ratings-image"] == "7"
    )
    error_message = "Keep the sidecar base and independent, explicit Charmhub charm/resource pins."
  }

  assert {
    condition     = length(output.application.config) == 0
    error_message = "Do not inject a staging database, hostname, or platform configuration into the reusable module."
  }

  assert {
    condition = output.requires == {
      database      = "database"
      logging       = "logging"
      traefik_route = "traefik-route"
    }
    error_message = "Export exactly the requires endpoints declared in charmcraft.yaml, not peers or nonexistent provided endpoints."
  }
}

run "custom_application_plan" {
  command = plan

  variables {
    app_name    = "ratings-preview"
    charm_name  = "ratings"
    model_uuid  = "00000000-0000-4000-8000-000000000002"
    units       = 3
    constraints = "arch=amd64 mem=2G"
    config = {
      database-name     = "preview-ratings"
      snap-store-url    = "https://store.invalid/v2/"
      rust-log          = "info"
      external-hostname = "ratings.preview.example.com"
    }
  }

  assert {
    condition = (
      output.application.name == var.app_name &&
      output.application.model_uuid == var.model_uuid &&
      output.application.units == var.units &&
      output.application.constraints == var.constraints &&
      output.application.config == var.config &&
      one(output.application.charm).name == var.charm_name
    )
    error_message = "Pass caller-selected deployment settings and charm config through without environment-specific overrides."
  }
}

run "resource_only_revision_plan" {
  command = plan

  variables {
    image_revision = 8
  }

  assert {
    condition = (
      output.application.resources["ratings-image"] == "8" &&
      one(output.application.charm).revision == var.revision &&
      one(output.application.charm).channel == var.channel
    )
    error_message = "Changing the workload resource must not change the selected charm pin."
  }
}

run "charm_only_revision_plan" {
  command = plan

  variables {
    revision = 3
  }

  assert {
    condition = (
      one(output.application.charm).revision == 3 &&
      output.application.resources["ratings-image"] == tostring(var.image_revision)
    )
    error_message = "Changing the charm pin must retain the workload resource pin."
  }
}

run "reject_zero_image_revision" {
  command = plan

  variables {
    image_revision = 0
  }

  expect_failures = [var.image_revision]
}

run "reject_negative_image_revision" {
  command = plan

  variables {
    image_revision = -1
  }

  expect_failures = [var.image_revision]
}

run "reject_fractional_image_revision" {
  command = plan

  variables {
    image_revision = 0.5
  }

  expect_failures = [var.image_revision]
}

run "reject_negative_charm_revision" {
  command = plan

  variables {
    revision = -1
  }

  expect_failures = [var.revision]
}

run "reject_fractional_charm_revision" {
  command = plan

  variables {
    revision = 0.5
  }

  expect_failures = [var.revision]
}

run "reject_local_charm" {
  command = plan

  variables {
    charm_name = "./app-center-ratings_amd64.charm"
  }

  expect_failures = [var.charm_name]
}

run "reject_invalid_channel" {
  command = plan

  variables {
    channel = "edge"
  }

  expect_failures = [var.channel]
}

run "reject_zero_units" {
  command = plan

  variables {
    units = 0
  }

  expect_failures = [var.units]
}

run "reject_fractional_units" {
  command = plan

  variables {
    units = 1.5
  }

  expect_failures = [var.units]
}
