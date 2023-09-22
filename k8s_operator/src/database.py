#!/usr/bin/env python3
# Copyright 2023 Canonical
# See LICENSE file for licensing details.

"""Assist working with the Ratings service database."""

import logging

import psycopg

logger = logging.getLogger(__name__)


USERS_TABLE_SCHEMA = """
CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    client_hash CHAR(64) NOT NULL UNIQUE, -- sha256($MACHINE_ID$USER)
    created TIMESTAMP NOT NULL DEFAULT NOW(),
    last_seen TIMESTAMP NOT NULL
)
"""

VOTES_TABLE_SCHEMA = """
CREATE TABLE IF NOT EXISTS votes (
    id SERIAL PRIMARY KEY,
    created TIMESTAMP NOT NULL DEFAULT NOW(),
    user_id_fk INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    snap_id CHAR(32) NOT NULL,
    snap_revision INT NOT NULL CHECK (snap_revision > 0),
    vote_up BOOLEAN NOT NULL
)
"""

USER_SNAP_VOTE_INDEX = """
CREATE UNIQUE INDEX IF NOT EXISTS idx_votes_unique_user_snap ON votes (
    user_id_fk,
    snap_id,
    snap_revision
)
"""


class DatabaseInitialisationError(Exception):
    """Raise when database cannot be properly initialised."""

    pass


class DatabaseConnectionError(Exception):
    """Raise when database cannot be connected to."""

    pass


class RatingsDatabase:
    """Interact with the Ratings service database."""

    def __init__(self, connection_string):
        self.connection_string = connection_string

    def create_tables(self):
        """Create the tables required for the ratings service."""
        try:
            connection = psycopg.connect(self.connection_string)
        except psycopg.Error:
            raise DatabaseConnectionError

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
            raise DatabaseInitialisationError
        finally:
            connection.close()

        logger.info("Database tables created successfully")
        connection.close()

    def ready(self) -> bool:
        """Report if the database is initialised correctly."""
        try:
            connection = psycopg.connect(self.connection_string)
            cursor = connection.cursor()
            return self._indexes_exist(cursor) and self._tables_exist(cursor)
        finally:
            connection.close()

    def _tables_exist(self, cursor) -> bool:
        """Report if database tables have been created."""
        records = cursor.execute("SELECT * FROM pg_catalog.pg_tables;").fetchall()
        tables = [t[1] for t in records if t[0] == "public"]
        return {"votes", "users"}.issubset(tables)

    def _indexes_exist(self, cursor) -> bool:
        """Report if database indexes have been created."""
        votes_indexes = cursor.execute(
            "SELECT indexname FROM pg_indexes WHERE tablename = (%s)",
            ("votes",),
        ).fetchall()

        if votes_indexes is None:
            return False

        index_names = [i[0] for i in votes_indexes]
        return "idx_votes_unique_user_snap" in index_names
