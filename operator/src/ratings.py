#!/usr/bin/env python3
# Copyright 2023 Canonical
# See LICENSE file for licensing details.

"""Initialise and interact with the ratings service."""

import logging

from database import RatingsDatabase

logger = logging.getLogger(__name__)


class Ratings:
    """Represents the Ratings application."""

    def __init__(self, connection_string: str, jwt_secret: str):
        self.connection_string = connection_string
        self.jwt_secret = jwt_secret
        self._database = RatingsDatabase(connection_string)

    def ready(self):
        """Report whether Ratings is ready to start."""
        if not (db_ready := self._database.ready()):
            logger.warning("Initialising database tables")
            self._database.create_tables()

        if not (jwt_secret_present := len(self.jwt_secret) > 0):
            logger.warning("Ratings service JWT token has zero-length")

        return db_ready and jwt_secret_present

    def pebble_layer(self) -> dict:
        """Return a dictionary representing a Pebble layer."""
        return {
            "summary": "ratings layer",
            "description": "pebble config layer for ratings",
            "services": {
                "ratings": {
                    "override": "replace",
                    "summary": "ratings",
                    "command": "/bin/ratings",
                    "startup": "enabled",
                    "environment": {
                        "APP_ENV": "dev",
                        "APP_JWT_SECRET": self.jwt_secret,
                        "APP_POSTGRES_URI": self.connection_string,
                        "APP_LOG_LEVEL": "info",
                    },
                }
            },
        }
