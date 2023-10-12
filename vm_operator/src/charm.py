#!/usr/bin/env python3
# Copyright 2023 Canonical
# See LICENSE file for licensing details.

"""Ubuntu Software Centre ratings service.

A backend service to support application ratings in the new Ubuntu Software Centre.
"""

import logging
import os
import secrets
from os import environ
from pathlib import Path

import ops
from charms.data_platform_libs.v0.data_interfaces import DatabaseCreatedEvent, DatabaseRequires
from charms.operator_libs_linux.v1 import snap
from ops.model import ActiveStatus, MaintenanceStatus
from ratings import Ratings

logger = logging.getLogger(__name__)


PATH = Path("/srv/app")
UNIT_PATH = Path("/etc/systemd/system/ratings.service")
CARGO_PATH = Path(environ.get("HOME", "/root")) / ".cargo/bin/cargo"
PORT = 443
NAME = "ratings"
HOST = "0.0.0.0"


class RatingsCharm(ops.CharmBase):
    """Main operator class for ratings service."""

    def __init__(self, *args):
        super().__init__(*args)
        self._ratings = Ratings()

        # Initialise the integration with PostgreSQL
        self._database = DatabaseRequires(self, relation_name="database", database_name="ratings")

        # Observe common Juju events
        self.framework.observe(self._database.on.database_created, self._on_database_created)
        self.framework.observe(self.on.install, self._on_install)
        self.framework.observe(self.on.start, self._on_start)
        self.framework.observe(self.on.upgrade_charm, self._on_upgrade_charm)

    def _on_start(self, _):
        """Start Ratings."""
        self._ratings.start()
        self.unit.status = ActiveStatus()

    def _on_upgrade_charm(self, _):
        """Ensure the snap is refreshed (in channel) if there are new revisions."""
        self.unit.status = ops.MaintenanceStatus("refreshing Ratings")
        try:
            self._ratings.refresh()
        except snap.SnapError as e:
            self.unit.status = ops.BlockedStatus(str(e))

    def _on_install(self, _):
        """Install prerequisites for the application."""
        self.unit.status = MaintenanceStatus("Installing Ratings")

        try:
            self._ratings.install()
            self.unit.status = MaintenanceStatus("Installation complete, waiting for database.")
        except snap.SnapError as e:
            logger.error(f"Failed to install Ratings via snap: {e}")
            self.unit.status = ops.BlockedStatus(str(e))

    def _on_database_created(self, _: DatabaseCreatedEvent):
        """Handle the database creation event."""
        logger.info("Database created event triggered.")
        self._update_service_config()

    def _update_service_config(self):
        """Update the service config and restart Ratings."""
        logger.info("Updating config and resterting Ratings.")

        if self.model.get_relation("database") is None:
            logger.warning("No database relation found. Waiting.")
            self.unit.status = ops.WaitingStatus("Waiting for database relation")
            return

        self.unit.status = ops.MaintenanceStatus("Attempting to update Ratings config.")
        # Get connection string from Juju relation to db
        connection_string = self._db_connection_string()

        # Generate jwt secret
        jwt_secret = self._jwt_secret()

        # Ensure squid proxy
        self._set_proxy()

        try:
            logger.info("Updating and resuming snap service for Ratings.")
            self._ratings.configure(
                jwt_secret=jwt_secret,
                postgres_uri=connection_string,
                migration_postgres_uri=connection_string,
                log_level=self.config["log-level"],
                env=self.config["env"],
            )
            self.unit.open_port(protocol="tcp", port=PORT)
            self.unit.status = ops.ActiveStatus()
            logger.info("Ratings service started successfully.")
        except Exception as e:
            logger.error(f"Failed to start Ratings service: {str(e)}")
            self.unit.status = ops.BlockedStatus(f"Failed to start Ratings service: {str(e)}")

    def _db_connection_string(self) -> str:
        """Report database connection string using info from relation databag."""
        logger.info("Attempting to generate database connection string.")

        relation = self.model.get_relation("database")

        if not relation:
            logger.warning("Database relation not found. Returning empty connection string.")
            return ""

        data = self._database.fetch_relation_data()[relation.id]
        username = data.get("username")
        password = data.get("password")
        endpoints = data.get("endpoints")

        if username and password and endpoints:
            connection_string = f"postgres://{username}:{password}@{endpoints}/ratings"
            logger.info(f"Generated database connection string with endpoints: {endpoints}.")
            return connection_string
        else:
            logger.warning("Missing database relation data. Cannot generate connection string.")
            return ""

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

    def _set_proxy(self):
        """Set Squid proxy environment variables if configured."""
        proxy_url = os.environ.get("JUJU_CHARM_HTTP_PROXY")
        if proxy_url:
            os.environ["HTTP_PROXY"] = proxy_url
            os.environ["HTTPS_PROXY"] = proxy_url


if __name__ == "__main__":  # pragma: nocover
    ops.main(RatingsCharm)
