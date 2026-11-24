# Copyright 2026 Canonical Ltd.
# See LICENSE file for licensing details.

variable "model_uuid" {
  description = "UUID of the existing Juju Kubernetes model in which to deploy Ratings."
  type        = string
  nullable    = false
}

variable "app_name" {
  description = "Name of the application in the Juju model."
  type        = string
  default     = "ratings"
  nullable    = false
}

variable "charm_name" {
  description = "Verified published Charmhub name for the Ratings sidecar charm. Override only for a compatible published charm."
  type        = string
  default     = "app-center-ratings"
  nullable    = false

  validation {
    condition     = can(regex("^[a-z][a-z0-9-]*$", var.charm_name))
    error_message = "Supply a published Charmhub charm name, not a local archive path or URL."
  }
}

variable "channel" {
  description = "Approved Charmhub channel (track/risk or track/risk/branch). No release channel is assumed."
  type        = string
  nullable    = false

  validation {
    condition     = can(regex("^[^/[:space:]]+/(stable|candidate|beta|edge)(/[^/[:space:]]+)?$", var.channel))
    error_message = "Specify a channel as track/risk or track/risk/branch."
  }
}

variable "revision" {
  description = "Explicit tested Charmhub charm revision, independent of the workload resource revision."
  type        = number
  nullable    = false

  validation {
    condition     = var.revision >= 0 && floor(var.revision) == var.revision
    error_message = "Pin a non-negative integer charm revision."
  }
}

variable "image_revision" {
  description = "Explicit positive Charmhub ratings-image OCI resource revision. Juju provider 1.5.7 rejects resource revision zero."
  type        = number
  nullable    = false

  validation {
    condition     = var.image_revision > 0 && floor(var.image_revision) == var.image_revision
    error_message = "Pin a positive integer Charmhub ratings-image resource revision (> 0), not an image URL, tag, or digest."
  }
}

variable "units" {
  description = "Number of units to deploy; one unit is not highly available."
  type        = number
  default     = 1
  nullable    = false

  validation {
    condition     = var.units >= 1 && floor(var.units) == var.units
    error_message = "Use a positive integer replica count."
  }
}

variable "constraints" {
  description = "Juju constraints for this application. The current Ratings rock and charm target amd64."
  type        = string
  default     = "arch=amd64"
  nullable    = false
}

variable "config" {
  description = "Ratings charm configuration: database-name, snap-store-url, rust-log, and external-hostname. Defaults belong to the charm; no environment-specific database or hostname is selected here."
  type        = map(string)
  default     = {}
  nullable    = false
}
