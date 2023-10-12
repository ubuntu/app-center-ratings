import os
import unittest
from unittest import mock
from unittest.mock import patch

from charm import RatingsCharm
from charms.data_platform_libs.v0.data_interfaces import DatabaseCreatedEvent
from ops.model import ActiveStatus, MaintenanceStatus, WaitingStatus
from ops.testing import Harness


class MockDatabaseEvent:
    def __init__(self, id, name="database"):
        self.name = name
        self.id = id


DB_RELATION_DATA = {
    "database": "ratings",
    "endpoints": "postgres:5432",
    "password": "password",
    "username": "username",
    "version": "14.8",
}


class TestCharm(unittest.TestCase):
    def setUp(self):
        self.harness = Harness(RatingsCharm)
        self.addCleanup(self.harness.cleanup)
        self.harness.begin()

    @mock.patch("charm.Ratings.install")
    def test_on_install(self, _install):
        self.harness.charm.on.install.emit()
        self.assertEqual(
            self.harness.charm.unit.status,
            MaintenanceStatus("Installation complete, waiting for database."),
        )

    @patch("charm.Ratings.refresh", lambda _: True)
    def test_upgrade_charm(self):
        self.harness.charm.on.upgrade_charm.emit()
        self.assertEqual(self.harness.charm.unit.status, MaintenanceStatus("refreshing Ratings"))

    @mock.patch("charm.Ratings.start")
    def test_on_start(self, _resume):
        # Run the handler
        self.harness.charm.on.start.emit()
        # Ensure we set an ActiveStatus for the charm
        self.assertEqual(self.harness.charm.unit.status, ActiveStatus())

    @mock.patch("charm.RatingsCharm._update_service_config")
    def test_on_database_created(self, _update):
        # Create a mock DatabaseCreatedEvent
        mock_event = mock.MagicMock(spec=DatabaseCreatedEvent)

        # Simulate database created event
        self.harness.charm._on_database_created(mock_event)

        # Check _update_service_config was called
        _update.assert_called_once()

    def test_ratings_db_connection_string_no_relation(self):
        self.assertEqual(self.harness.charm._db_connection_string(), "")

    @patch("charm.DatabaseRequires.fetch_relation_data", lambda x: {0: DB_RELATION_DATA})
    def test_ratings_db_connection_string(self):
        self.harness.add_relation("database", "postgresql", unit_data=DB_RELATION_DATA)
        expected = "postgres://username:password@postgres:5432/ratings"
        self.assertEqual(self.harness.charm._db_connection_string(), expected)

    @patch("charm.RatingsCharm._update_service_config")
    @patch("charm.DatabaseRequires.is_resource_created", lambda x: True)
    def test_ratings_database_created_database_success(self, _update):
        rel_id = self.harness.add_relation("database", "postgresql", unit_data=DB_RELATION_DATA)
        self.harness.charm._database.on.database_created.emit(MockDatabaseEvent(id=rel_id))

        _update.assert_called_once()

    @patch("charm.RatingsCharm._set_proxy")
    @patch("charm.RatingsCharm._db_connection_string", return_value="bar")
    @patch("charm.RatingsCharm._jwt_secret", return_value="foo")
    @mock.patch("charm.Ratings.configure")
    def test_update_service_config(self, _conf, _jwt, _db_string, _proxy):
        # Set env and log-level
        self.harness.update_config({"env": "test-env", "log-level": "debug"})

        # If no relation, wait on relation
        self.harness.charm._update_service_config()
        self.assertEqual(
            self.harness.charm.unit.status, WaitingStatus("Waiting for database relation")
        )

        # If the relation is set, open the ports and restart the service
        self.harness.add_relation("database", "postgresql", unit_data=DB_RELATION_DATA)
        self.harness.charm._update_service_config()

        # JWT was generated
        _jwt.assert_called_once()

        # Connection string retrieved
        _db_string.assert_called_once()

        # Proxy set
        _proxy.assert_called_once()

        # Configure is called with the correct values
        _conf.assert_called_with(
            jwt_secret="foo",
            postgres_uri="bar",
            migration_postgres_uri="bar",
            log_level="debug",
            env="test-env",
        )

        # Check the ports have been opened
        opened_ports = {(p.protocol, p.port) for p in self.harness.charm.unit.opened_ports()}
        self.assertEqual(opened_ports, {("tcp", 443)})

        # Check status is active
        self.assertEqual(self.harness.charm.unit.status, ActiveStatus())

    def test_ratings_jwt_secret_from_peer_data(self):
        content = {"jwt-secret": "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"}
        secret_id = self.harness.add_model_secret(owner=self.harness.charm.app, content=content)
        self.harness.add_relation(
            "ratings-peers", "ubuntu-software-ratings", app_data={"jwt-secret-id": secret_id}
        )
        secret = self.harness.charm._jwt_secret()
        self.assertEqual(secret, content["jwt-secret"])

    def test_ratings_jwt_secret_no_relation(self):
        new_secret = self.harness.charm._jwt_secret()
        self.assertEqual(new_secret, "")

    def test_ratings_jwt_secret_create(self):
        self.harness.add_relation("ratings-peers", "ubuntu-software-ratings")
        self.harness.set_leader(True)
        new_secret = self.harness.charm._jwt_secret()
        self.assertEqual(len(new_secret), 48)

    @mock.patch.dict(os.environ, {"JUJU_CHARM_HTTP_PROXY": "http://example.com:3128"}, clear=True)
    def test_set_proxy(self):
        # Call the method
        self.harness.charm._set_proxy()

        # Assert that the environment variables were set
        self.assertEqual(os.environ["HTTP_PROXY"], "http://example.com:3128")
        self.assertEqual(os.environ["HTTPS_PROXY"], "http://example.com:3128")

        with mock.patch.dict(os.environ, {}, clear=True):
            self.harness.charm._set_proxy()

            # Assert that the environment variables were not set
            self.assertNotIn("HTTP_PROXY", os.environ)
            self.assertNotIn("HTTPS_PROXY", os.environ)
