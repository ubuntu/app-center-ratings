# Copyright 2026 Canonical Ltd.
# See LICENSE file for licensing details.

output "application" {
  description = "Object representing the deployed Ratings application."
  value       = juju_application.ratings
}

output "requires" {
  description = "Required relation endpoints declared by the Ratings charm; optional endpoints are not automatically integrated."
  value = {
    database      = "database"
    logging       = "logging"
    traefik_route = "traefik-route"
  }
}
