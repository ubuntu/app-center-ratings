#!/usr/bin/env python3
# Copyright 2023 Canonical
# See LICENSE file for licensing details.

"""Ubuntu Software Centre ratings service.

A backend service to support application ratings in the new Ubuntu Software Centre.
"""

import logging

import ops
from charms.data_platform_libs.v0.data_interfaces import DatabaseCreatedEvent, DatabaseRequires
from ratings import Ratings

logger = logging.getLogger(__name__)


class RatingsCharm(ops.CharmBase):
    """Main operator class for ratings service."""

    def __init__(self, *args):
        super().__init__(*args)

        self._container = self.unit.get_container("ratings")
        self._database = DatabaseRequires(self, relation_name="database", database_name="ratings")
        self._ratings = None

        if self._database.is_resource_created():
            connection_string = self._db_connection_string()
            self._ratings = Ratings(connection_string)

        self.framework.observe(self.on.ratings_pebble_ready, self._on_ratings_pebble_ready)
        self.framework.observe(self._database.on.database_created, self._on_database_created)

    def _on_ratings_pebble_ready(self, _: ops.PebbleReadyEvent):
        """Define and start the workload using the Pebble API."""
        self._start_ratings()

    def _on_database_created(self, _: DatabaseCreatedEvent):
        """Handle the database creation event."""
        if not self._ratings.database_initialised():
            self.unit.status = ops.MaintenanceStatus("Initialising database")

            if not self._ratings.create_database_tables():
                self.unit.status = ops.BlockedStatus("Failed to create database tables")
                return

        self._start_ratings()

    def _start_ratings(self):
        """Populate Pebble layer and start the ratings service."""
        if self.model.get_relation("database") is None:
            self.unit.status = ops.WaitingStatus("Waiting for database relation")
            return

        if self._ratings is None or not self._ratings.database_initialised():
            self.unit.status = ops.WaitingStatus("Waiting for database initialisation")
            return

        if self._container.can_connect():
            self._container.add_layer("ratings", self._ratings.pebble_layer(), combine=True)
            self._container.replan()
            self.unit.status = ops.ActiveStatus()
        else:
            self.unit.status = ops.WaitingStatus("Waiting for ratings container")

    def _db_connection_string(self) -> str:
        """Report database connection string using info from relation databag."""
        relation_id = self._database.relations[0].id
        data = self._database.fetch_relation_data()[relation_id]

        username = data.get("username")
        password = data.get("password")
        endpoints = data.get("endpoints")

        return f"postgres://{username}:{password}@{endpoints}/ratings"


if __name__ == "__main__":  # pragma: nocover
    ops.main(RatingsCharm)
