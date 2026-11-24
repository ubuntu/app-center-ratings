# Copyright 2026 Canonical Ltd.
# See LICENSE file for licensing details.

terraform {
  required_version = ">= 1.7.0"
  required_providers {
    juju = {
      source  = "juju/juju"
      version = "~> 1.5.7"
    }
  }
}
