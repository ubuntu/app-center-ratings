import unittest

import logging
from pathlib import Path
from unittest import mock
from unittest.mock import Mock, call, mock_open, patch
from charms.data_platform_libs.v0.data_interfaces import DatabaseCreatedEvent
from charm import RatingsCharm, UNIT_PATH, APP_PATH, CARGO_PATH
from charms.operator_libs_linux.v0 import apt, systemd
from ops.model import ActiveStatus, BlockedStatus, MaintenanceStatus, WaitingStatus
import os
from os import environ
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

RENDERED_SYSTEMD_UNIT = """[Unit]
Description=App Center Ratings Service
After=network.target

[Service]
Environment="APP_ENV=dev"
Environment="APP_JWT_SECRET="
Environment="APP_LOG_LEVEL=info"
Environment="APP_POSTGRES_URI="
Environment="APP_MIGRATION_POSTGRES_URI="
WorkingDirectory = /srv/app
Restart = always
RestartSec = 5
ExecStart=/srv/app/target/release/ratings
ExecReload = /bin/kill -s HUP $MAINPID
ExecStop = /bin/kill -s TERM $MAINPID
ExecStartPre = /bin/mkdir /srv/app/run
PIDFile = /srv/app/run/hello-juju.pid
ExecStopPost = /bin/rm -rf /srv/app/run

[Install]
WantedBy = multi-user.target"""

class TestCharm(unittest.TestCase):
    def setUp(self):
        self.harness = Harness(RatingsCharm)
        self.addCleanup(self.harness.cleanup)
        self.harness.begin()

    @mock.patch("charm.RatingsCharm._install_apt_packages")
    def test_on_install(self, _install):
        self.harness.charm.on.install.emit()
        self.assertEqual(
            self.harness.charm.unit.status, MaintenanceStatus("Installation complete, waiting for database.")
        )
        _install.assert_called_with(["curl", "git", "gcc", "libssl-dev", "pkg-config","protobuf-compiler"])

    @mock.patch("charms.operator_libs_linux.v0.systemd.service_resume")
    def test_on_start(self, _resume):
        # Run the handler
        self.harness.charm.on.start.emit()
        # Ensure we set an ActiveStatus for the charm
        self.assertEqual(self.harness.charm.unit.status, ActiveStatus())
        _resume.assert_called_with("ratings")

    @mock.patch("charms.operator_libs_linux.v0.systemd.daemon_reload")
    @mock.patch("os.chmod")
    def test_render_systemd_unit(self, _chmod, _reload):
        # Create a mock for the `open` method, set the return value of `read` to
        # the contents of the systemd unit template
        with open("templates/ratings-service.j2", "r") as f:
            m = mock_open(read_data=f.read())

        # Patch the `open` method with our mock
        with patch("builtins.open", m, create=True):
            # Ensure the stored value is clear to test it's set properly
            self.harness.charm._stored.port = ""
            # Mock the return value of the `check_call`
            _reload.return_value = 0
            # Call the method
            self.harness.charm._render_systemd_unit()

        # Check the unit path is correct
        self.assertEqual(UNIT_PATH, Path("/etc/systemd/system/ratings.service"))
        # Check the template is opened read-only in the first call to open
        self.assertEqual(m.call_args_list[0][0], ("templates/ratings-service.j2", "r"))
        # Check the systemd unit file is opened with "w+" mode in the second call to open
        self.assertEqual(m.call_args_list[1][0], (UNIT_PATH, "w+"))
        # Ensure the correct rendered template is written to file
        m.return_value.write.assert_called_with(RENDERED_SYSTEMD_UNIT)
        # Check the file permissions are set correctly
        _chmod.assert_called_with(UNIT_PATH, 0o755)
        # Check that systemd is reloaded to register the changes to the unit
        _reload.assert_called_once()

    @mock.patch.dict(os.environ, {}, clear=True)
    @mock.patch("charm.check_output")
    @mock.patch("charm.Repo.clone_from")
    @mock.patch("charm.Path")
    @mock.patch("shutil.rmtree")
    def test_setup_application(self, _rmtree, _path, _clone, _check):
        # Check that the app path was cleared/deleted
        _path.return_value.is_dir.return_value = True

        # Call the method
        self.harness.charm._setup_application()

        # Check that app-repo is set correctly
        self.assertEqual(self.harness.charm._stored.repo, self.harness.charm.config["app-repo"])

        self.assertEqual(APP_PATH, Path("/srv/app"))

        # Check squid proxy is set correctly
        self.assertEqual(os.environ.get("HTTP_PROXY"), "http://proxy.example.com")
        self.assertEqual(os.environ.get("HTTPS_PROXY"), "http://proxy.example.com")

        # Ensure we set the charm status correctly
        self.assertEqual(
            self.harness.charm.unit.status, MaintenanceStatus("Code fetched, building now.")
        )
        # Check we try to remove the directory
        _rmtree.assert_called_with("/srv/app")

        # Check we set the stored repository where none exists
        self.assertEqual(self.harness.charm._stored.repo, "https://github.com/matthew-hagemann/app-center-ratings")

        # Ensure we clone the repo
        _clone.assert_called_with("https://github.com/matthew-hagemann/app-center-ratings", APP_PATH,branch='vm-charm')

        # Check that cargo build was called correctly
        self.assertEqual(
            _check.call_args_list,
            [
                call([str(CARGO_PATH), "build", "--release"], cwd=APP_PATH),
            ],
        )

        # Make sure we don't try to remove the directory
        _path.return_value.is_dir.return_value = False
        self.harness.charm._stored.repo = "https://myrepo"
        _rmtree.reset_mock()

        # Call the method
        self.harness.charm._setup_application()
        _rmtree.assert_not_called()
        self.assertEqual(self.harness.charm._stored.repo, "https://myrepo")

    @mock.patch("charm.RatingsCharm._setup_application")
    @mock.patch("charm.RatingsCharm._render_systemd_unit")
    @mock.patch("charm.RatingsCharm._start_ratings")
    def test_on_database_created(self, _start, _render, _setup):
        self.harness.charm._stored.install_completed = True
        # Create a mock DatabaseCreatedEvent
        mock_event = mock.MagicMock(spec=DatabaseCreatedEvent)

        # Simulate database created event
        self.harness.charm._on_database_created(mock_event)

        # Check _render_systemd_unit was called
        _render.assert_called_once()
        # Check _start_ratings was called
        _start.assert_called_once()
        # Check _setup_application was called
        _setup.assert_called_once()

        # If not installed, don't call functions
        _render.reset_mock()
        _start.reset_mock()
        _setup.reset_mock()

        self.harness.charm._stored.install_completed = False

        # Simulate database created event
        self.harness.charm._on_database_created(mock_event)

        _render.assert_not_called()
        _start.assert_not_called()
        _setup.assert_not_called()

    @mock.patch("charms.operator_libs_linux.v0.apt.update")
    @mock.patch("charms.operator_libs_linux.v0.apt.add_package")
    def test_install_apt_packages(self, _add_package, _update):
        # Call the method with some packages to install
        self.harness.charm._install_apt_packages(["curl", "vim"])
        # Check that apt is called with the correct arguments
        _update.assert_called_once()
        _add_package.assert_called_with(["curl", "vim"])
        # Now check that if an exception is raised we do the right logging
        _add_package.reset_mock()
        _add_package.return_value = 1
        _add_package.side_effect = apt.PackageNotFoundError
        self.harness.charm._install_apt_packages(["curl", "vim"])
        self.assertEqual(
            self.harness.charm.unit.status, BlockedStatus("Failed to install packages")
        )
        # Now check that if an exception is raised we do the right logging
        _add_package.reset_mock()
        _add_package.return_value = 1
        _add_package.side_effect = apt.PackageError
        self.harness.charm._install_apt_packages(["curl", "vim"])
        self.assertEqual(
            self.harness.charm.unit.status, BlockedStatus("Failed to install packages")
        )

    def test_ratings_db_connection_string_no_relation(self):
        self.assertEqual(self.harness.charm._db_connection_string(), "")

    @patch("charm.DatabaseRequires.fetch_relation_data", lambda x: {0: DB_RELATION_DATA})
    def test_ratings_db_connection_string(self):
        self.harness.add_relation("database", "postgresql", unit_data=DB_RELATION_DATA)
        expected = "postgres://username:password@postgres:5432/ratings"
        self.assertEqual(self.harness.charm._db_connection_string(), expected)

    @patch("charm.DatabaseRequires.is_resource_created", lambda x: True)
    def test_ratings_database_created_database_success(self):
        rel_id = self.harness.add_relation("database", "postgresql", unit_data=DB_RELATION_DATA)
        self.harness.charm._database.on.database_created.emit(MockDatabaseEvent(id=rel_id))
        self.assertEqual(self.harness.model.unit.status, WaitingStatus("Waiting for install to complete."))

    @mock.patch("charms.operator_libs_linux.v0.systemd.service_resume")
    def test_start_ratings(self, _resume):
        # If no relation, wait on relation
        self.harness.charm._start_ratings()
        self.assertEqual(self.harness.charm.unit.status, WaitingStatus('Waiting for database relation'))

        # If the relation is set, open the ports and restart the service
        self.harness.add_relation("database", "postgresql", unit_data=DB_RELATION_DATA)
        self.harness.charm._start_ratings()

        # Check the service was resumed
        _resume.assert_called_with("ratings")

        # Check the ports have been opened
        opened_ports = {(p.protocol, p.port) for p in self.harness.charm.unit.opened_ports()}
        self.assertEqual(opened_ports, {('tcp', 443)})

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

    @mock.patch("charms.operator_libs_linux.v0.systemd.service_restart")
    @mock.patch("charm.check_output")
    @mock.patch("charm.Repo")
    def test_on_pull_and_rebuild(self, _MockRepo, _check, _restart):

        # Can't mock chain in the @mock.patch, so set up chaining manually for pull
        mock_pull = mock.Mock()
        mock_origin = mock.Mock(pull=mock_pull)
        mock_remotes = mock.Mock(origin=mock_origin)

        _MockRepo.return_value.remotes = mock_remotes  # Set up mock chaining

        # Create event mock
        mock_event = mock.Mock()
        mock_event.set_results = mock.Mock()
        mock_event.fail = mock.Mock()

        # Run the handler
        self.harness.charm._on_pull_and_rebuild(mock_event)

        # Check squid proxy is set correctly
        self.assertEqual(os.environ.get("HTTP_PROXY"), "http://proxy.example.com")
        self.assertEqual(os.environ.get("HTTPS_PROXY"), "http://proxy.example.com")

        # Ensure we clone the repo
        mock_pull.assert_called()

        # Check that cargo build was called correctly
        self.assertEqual(
            _check.call_args_list,
            [
                call([str(CARGO_PATH), "build", "--release"], cwd=APP_PATH),
            ],
        )

        _restart.assert_called_with("ratings")

        self.assertEqual(self.harness.charm.unit.status, ActiveStatus("Successfully pulled and rebuilt."))
