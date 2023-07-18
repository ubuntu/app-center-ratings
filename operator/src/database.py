#!/usr/bin/env python3
# Copyright 2023 Canonical
# See LICENSE file for licensing details.

"""Constants that define the database schema for the ratings service."""


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
