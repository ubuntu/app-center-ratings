#!/usr/bin/env python3
# Copyright 2023 Canonical
# See LICENSE file for licensing details.

"""Ubuntu Software Centre ratings service.

A backend service to support application ratings in the new Ubuntu Software Centre.
"""

import logging
import secrets

import ops
from charms.data_platform_libs.v0.data_interfaces import DatabaseCreatedEvent, DatabaseRequires
from charms.traefik_k8s.v2.ingress import IngressPerAppRequirer
from database import DatabaseConnectionError, DatabaseInitialisationError
from ratings import Ratings

logger = logging.getLogger(__name__)


class RatingsCharm(ops.CharmBase):
    """Main operator class for ratings service."""

    def __init__(self, *args):
        super().__init__(*args)
        self._container = self.unit.get_container("ratings")
        self._ratings_svc = None

        # Initialise the integration with Ingress providers (Traefik/nginx)
        self._ingress = IngressPerAppRequirer(
            self,
            host=f"{self.app.name}.{self.model.name}.svc.cluster.local",
            port=18080,
            scheme=lambda: "h2c",
        )

        # Initialise the integration with PostgreSQL
        self._database = DatabaseRequires(self, relation_name="database", database_name="ratings")

        self.framework.observe(self.on.ratings_pebble_ready, self._on_ratings_pebble_ready)
        self.framework.observe(self._database.on.database_created, self._on_database_created)

    def _on_ratings_pebble_ready(self, _: ops.PebbleReadyEvent):
        """Define and start the workload using the Pebble API."""
        self._start_ratings()

    def _on_database_created(self, _: DatabaseCreatedEvent):
        """Handle the database creation event."""
        if not self._ratings:
            return

        try:
            if not self._ratings.ready():
                self.unit.status = ops.MaintenanceStatus("Initialising database")
        except (DatabaseConnectionError, DatabaseInitialisationError) as e:
            logger.error(str(e))
            self.unit.status = ops.BlockedStatus("Failed to create database tables")
            return

        self._start_ratings()

    def _start_ratings(self):
        """Populate Pebble layer and start the ratings service."""
        if self.model.get_relation("database") is None:
            self.unit.status = ops.WaitingStatus("Waiting for database relation")
            return

        if not (self._ratings and self._ratings.ready()):
            self.unit.status = ops.WaitingStatus("Ratings not yet initialised")
            return

        if self._container.can_connect():
            self._container.add_layer("ratings", self._ratings.pebble_layer(), combine=True)
            self._container.replan()
            self.unit.open_port(protocol="tcp", port=18080)
            self.unit.status = ops.ActiveStatus()
        else:
            self.unit.status = ops.WaitingStatus("Waiting for ratings container")

    @property
    def _ratings(self):
        """Ratings property that is truthy only when pre-conditions are met."""
        if self._ratings_svc:
            return self._ratings_svc

        if self._database.is_resource_created():
            connection_string = self._db_connection_string()
            jwt_secret = self._jwt_secret()
            self._ratings_svc = Ratings(connection_string, jwt_secret)

        return self._ratings_svc

    def _db_connection_string(self) -> str:
        """Report database connection string using info from relation databag."""
        relation = self.model.get_relation("database")
        if not relation:
            return ""

        data = self._database.fetch_relation_data()[relation.id]
        username = data.get("username")
        password = data.get("password")
        endpoints = data.get("endpoints")

        return f"postgres://{username}:{password}@{endpoints}/ratings"

    def _jwt_secret(self) -> str:
        """Report the apps JWT secret; create one if it doesn't exist."""
        # If the peer relation is not ready, just return an empty string
        relation = self.model.get_relation("ratings-peers")
        if not relation:
            return ""

        # If the secret already exists, grab its content and return it
        secret_id = relation.data[self.app].get("jwt-secret-id", None)
        if secret_id:
            secret = self.model.get_secret(id=secret_id)
            return secret.peek_content().get("jwt-secret")

        if self.unit.is_leader():
            logger.info("Creating a new JWT secret")
            content = {"jwt-secret": secrets.token_hex(24)}
            secret = self.app.add_secret(content)
            # Store the secret id in the peer relation for other units if required
            relation.data[self.app]["jwt-secret-id"] = secret.id
            return content["jwt-secret"]
        else:
            return ""


if __name__ == "__main__":  # pragma: nocover
    ops.main(RatingsCharm)
