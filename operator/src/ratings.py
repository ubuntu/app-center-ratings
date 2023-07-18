#!/usr/bin/env python3
# Copyright 2023 Canonical
# See LICENSE file for licensing details.

"""Initialise and interact with the ratings service."""

import logging

import psycopg
from database import USER_SNAP_VOTE_INDEX, USERS_TABLE_SCHEMA, VOTES_TABLE_SCHEMA

logger = logging.getLogger(__name__)


class Ratings:
    """Represents the Ratings application."""

    def __init__(self, connection_string: str):
        self.connection_string = connection_string

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
                        "ENV": "dev",
                        # TODO: Replace this placeholder
                        "JWT_SECRET": "deadbeef",
                        "POSTGRES": self.connection_string,
                        "RUST_LOG": "info",
                    },
                }
            },
        }

    def create_database_tables(self) -> bool:
        """Create the tables required for the ratings service."""
        try:
            connection = psycopg.connect(self.connection_string)
        except psycopg.Error as e:
            logger.error("Could not connect to database: %s", str(e.diag.message_primary))
            return False

        logger.info("Creating database schema")
        try:
            with connection.cursor() as cur:
                cur.execute(USERS_TABLE_SCHEMA)
                cur.execute(VOTES_TABLE_SCHEMA)
                cur.execute(USER_SNAP_VOTE_INDEX)
            connection.commit()
        except psycopg.Error as e:
            logger.error(
                "Could not commit database schema changes: %s", str(e.diag.message_primary)
            )
            return False
        finally:
            connection.close()

        logger.info("Database tables created successfully")
        connection.close()
        return True

    def database_initialised(self) -> bool:
        """Report if the database is initialised correctly."""
        try:
            connection = psycopg.connect(self.connection_string)
            cursor = connection.cursor()
            return self._database_indexes_exist(cursor) and self._database_tables_exist(cursor)
        finally:
            connection.close()

    def _database_tables_exist(self, cursor) -> bool:
        """Report if database tables have been created."""
        records = cursor.execute("SELECT * FROM pg_catalog.pg_tables;").fetchall()
        tables = [t[1] for t in records if t[0] == "public"]
        return {"votes", "users"}.issubset(tables)

    def _database_indexes_exist(self, cursor) -> bool:
        """Report if database indexes have been created."""
        votes_indexes = cursor.execute(
            "SELECT indexname FROM pg_indexes WHERE tablename = (%s)",
            ("votes",),
        ).fetchall()

        if votes_indexes is None:
            return False

        index_names = [i[0] for i in votes_indexes]
        return "idx_votes_unique_user_snap" in index_names
