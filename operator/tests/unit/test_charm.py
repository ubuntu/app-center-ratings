# Copyright 2023 Canonical
# See LICENSE file for licensing details.

import unittest
from types import SimpleNamespace
from unittest.mock import patch

import ops
import ops.testing
from charm import RatingsCharm
from database import DatabaseInitialisationError
from ratings import Ratings

DB_RELATION_DATA = {
    "database": "ratings",
    "endpoints": "postgres:5432",
    "password": "password",
    "username": "username",
    "version": "14.8",
}

MOCK_RATINGS = Ratings("postgres://username:password@postgres:5432/ratings", "deadbeef")


class MockDatabaseEvent:
    def __init__(self, id, name="database"):
        self.name = name
        self.id = id


class TestCharm(unittest.TestCase):
    def setUp(self):
        self.harness = ops.testing.Harness(RatingsCharm)
        self.addCleanup(self.harness.cleanup)
        self.harness.set_leader(True)
        self.harness.begin()

    def test_ratings_pebble_ready_no_relation(self):
        expected_plan = {}
        self.harness.container_pebble_ready("ratings")
        updated_plan = self.harness.get_container_pebble_plan("ratings").to_dict()
        self.assertEqual(expected_plan, updated_plan)
        self.assertEqual(
            self.harness.model.unit.status, ops.WaitingStatus("Waiting for database relation")
        )

    def test_ratings_sets_database_name_on_database_relation(self):
        rel_id = self.harness.add_relation("database", "postgresql", unit_data=DB_RELATION_DATA)
        self.harness.container_pebble_ready("ratings")
        app_data = self.harness.get_relation_data(rel_id, self.harness.charm.app.name)
        self.assertEqual(app_data, {"database": "ratings"})

    def test_ratings_pebble_ready_waits_for_db_initialisation(self):
        self.harness.add_relation("database", "postgresql", unit_data=DB_RELATION_DATA)
        self.harness.container_pebble_ready("ratings")
        self.assertEqual(
            self.harness.model.unit.status, ops.WaitingStatus("Ratings not yet initialised")
        )

    @patch("charm.RatingsCharm._ratings", MOCK_RATINGS)
    @patch("ratings.Ratings.ready", lambda x: True)
    def test_ratings_pebble_ready_sets_correct_plan(self):
        self.harness.add_relation("database", "postgresql", unit_data=DB_RELATION_DATA)
        self.harness.container_pebble_ready("ratings")
        self.assertEqual(self.harness.model.unit.status, ops.ActiveStatus())
        expected = {
            "services": {
                "ratings": {
                    "override": "replace",
                    "summary": "ratings",
                    "command": "/bin/ratings",
                    "startup": "enabled",
                    "environment": {
                        "APP_POSTGRES_URI": "postgres://username:password@postgres:5432/ratings",
                        "APP_JWT_SECRET": "deadbeef",
                        "APP_LOG_LEVEL": "info",
                        "APP_ENV": "dev",
                    },
                }
            },
        }
        self.assertEqual(self.harness.get_container_pebble_plan("ratings").to_dict(), expected)

    @patch("charm.RatingsCharm._ratings", MOCK_RATINGS)
    @patch("ratings.Ratings.ready", lambda x: True)
    def test_ratings_pebble_ready_waits_for_container(self):
        self.harness.add_relation("database", "postgresql", unit_data=DB_RELATION_DATA)
        self.harness.set_can_connect("ratings", False)
        self.harness.charm.on.ratings_pebble_ready.emit(SimpleNamespace(workload="foo"))
        self.assertEqual(
            self.harness.model.unit.status, ops.WaitingStatus("Waiting for ratings container")
        )

    def test_ratings_database_created_ratings_not_initialised(self):
        rel_id = self.harness.add_relation("database", "postgresql", unit_data=DB_RELATION_DATA)
        self.harness.charm._database.on.database_created.emit(MockDatabaseEvent(id=rel_id))
        self.harness.set_can_connect("ratings", True)
        plan = self.harness.get_container_pebble_plan("ratings").to_dict()
        self.assertEqual(plan, {})

    @patch("charm.DatabaseRequires.is_resource_created", lambda x: True)
    @patch("ratings.Ratings.ready")
    def test_ratings_database_created_database_not_initialised_fail_create_tables(self, db_init):
        rel_id = self.harness.add_relation("database", "postgresql", unit_data=DB_RELATION_DATA)
        db_init.side_effect = DatabaseInitialisationError
        self.harness.charm._database.on.database_created.emit(MockDatabaseEvent(id=rel_id))
        self.assertEqual(
            self.harness.model.unit.status, ops.BlockedStatus("Failed to create database tables")
        )

    @patch("charm.DatabaseRequires.is_resource_created", lambda x: True)
    @patch("ratings.Ratings.ready", lambda x: True)
    def test_ratings_database_created_database_success(self):
        rel_id = self.harness.add_relation("database", "postgresql", unit_data=DB_RELATION_DATA)
        self.harness.set_can_connect("ratings", True)
        self.harness.charm._database.on.database_created.emit(MockDatabaseEvent(id=rel_id))
        self.assertEqual(self.harness.model.unit.status, ops.ActiveStatus())

    def test_ratings_db_connection_string_no_relation(self):
        self.assertEqual(self.harness.charm._db_connection_string(), "")

    @patch("charm.DatabaseRequires.fetch_relation_data", lambda x: {0: DB_RELATION_DATA})
    def test_ratings_db_connection_string(self):
        self.harness.add_relation("database", "postgresql", unit_data=DB_RELATION_DATA)
        expected = "postgres://username:password@postgres:5432/ratings"
        self.assertEqual(self.harness.charm._db_connection_string(), expected)

    def test_ratings_jwt_secret_no_relation(self):
        new_secret = self.harness.charm._jwt_secret()
        self.assertEqual(new_secret, "")

    def test_ratings_jwt_secret_create(self):
        self.harness.add_relation("ratings-peers", "ubuntu-software-ratings")
        new_secret = self.harness.charm._jwt_secret()
        self.assertEqual(len(new_secret), 48)

    def test_ratings_jwt_secret_from_peer_data(self):
        content = {"jwt-secret": "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"}
        secret_id = self.harness.add_model_secret(owner=self.harness.charm.app, content=content)
        self.harness.add_relation(
            "ratings-peers", "ubuntu-software-ratings", app_data={"jwt-secret-id": secret_id}
        )
        secret = self.harness.charm._jwt_secret()
        self.assertEqual(secret, content["jwt-secret"])

    def test_ratings_property_already_initialised(self):
        self.harness.charm._ratings_svc = "foobar"
        self.assertEqual(self.harness.charm._ratings, "foobar")

    @patch("charm.DatabaseRequires.is_resource_created", lambda x: True)
    @patch("charm.DatabaseRequires.fetch_relation_data", lambda x: {0: DB_RELATION_DATA})
    def test_ratings_property_not_initialised(self):
        self.assertIsInstance(self.harness.charm._ratings, Ratings)
