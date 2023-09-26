#!/usr/bin/env python3
# Copyright 2023 Canonical
# See LICENSE file for licensing details.

"""Ubuntu Software Centre ratings service.

A backend service to support application ratings in the new Ubuntu Software Centre.
"""

import logging
import secrets

import ops

from pathlib import Path
from git import Repo
import shutil
from charms.operator_libs_linux.v0 import apt, systemd
from charms.data_platform_libs.v0.data_interfaces import DatabaseCreatedEvent, DatabaseRequires
from ops.model import MaintenanceStatus, BlockedStatus, ActiveStatus
from ops.framework import StoredState
from subprocess import check_output, CalledProcessError
import os
from os import environ
from jinja2 import Template

logger = logging.getLogger(__name__)


APP_PATH = Path("/srv/app")
UNIT_PATH = Path("/etc/systemd/system/ratings.service")
CARGO_PATH = Path(environ.get("HOME", "/root")) / ".cargo/bin/cargo"


class RatingsCharm(ops.CharmBase):
    """Main operator class for ratings service."""

    _stored = StoredState()

    def __init__(self, *args):
        super().__init__(*args)
        self._ratings_svc = None

        # Initialise the integration with PostgreSQL
        self._database = DatabaseRequires(self, relation_name="database", database_name="ratings")

        self.framework.observe(self._database.on.database_created, self._on_database_created)
        self.framework.observe(self.on.install, self._on_install)
        self._stored.set_default(repo="", port="", conn_str="")
        self.framework.observe(self.on.start, self._on_start)
        self.framework.observe(self.on.pull_and_rebuild_action, self._on_pull_and_rebuild)

        # self.framework.observe(self.on.config_changed, self._on_config_changed)
    def _on_start(self, _):
        """Start the workload"""
        # Enable and start the "ratings" systemd unit
        systemd.service_resume("ratings")
        self.unit.status = ActiveStatus()

    def _on_install(self, _):
        """Install prerequisites for the application"""

        self.unit.status = MaintenanceStatus("Installing rustc, cargo and other dependencies")

        # Install what is avalible via apt
        self._install_apt_packages(["curl", "git", "gcc", "libssl-dev", "pkg-config","protobuf-compiler"])

        # Ensure squid proxy, done after apt to not interfere
        proxy_url = self.config["squid-proxy-url"]
        if proxy_url:
            os.environ['HTTP_PROXY'] = proxy_url
            os.environ['HTTPS_PROXY'] = proxy_url


        # curl minial rust toolchain
        try:
            check_output("curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal", shell=True)

        except CalledProcessError as e:
            logger.error(f"Curl command failed with error code {e.returncode}")
            self.unit.status = BlockedStatus("Curl command failed")
            return


    def _render_systemd_unit(self):
        with open("templates/ratings-service.j2", "r") as t:
            template = Template(t.read())

        # Get connection string from Juju relation to db
        connection_string = self._db_connection_string()

        # Generate jwt secret
        jwt_secret = self._jwt_secret()
        rendered = template.render(
            project_root=APP_PATH,
            app_env=self.config["app-env"],
            app_host=self.config["app-host"],
            app_jwt_secret=jwt_secret,
            app_log_level=self.config["app-log-level"],
            app_name=self.config["app-name"],
            app_port=self.config["app-port"],
            app_postgres_uri=connection_string,
            app_migration_postgres_uri=connection_string
        )
        with open(UNIT_PATH, "w+") as t:
            t.write(rendered)

        os.chmod(UNIT_PATH, 0o755)
        systemd.daemon_reload()

    def _setup_application(self, _=None):
        """Clone Rust application into place and setup it's dependencies"""
        self.unit.status = MaintenanceStatus("fetching application code")

        # Delete the application directory if it exists already
        if Path(APP_PATH).is_dir():
            shutil.rmtree("/srv/app")

        # If this is the first time, set the repo in the stored state
        if not self._stored.repo:
            self._stored.repo = self.config["app-repo"]

        # Ensure squid proxy
        proxy_url = self.config["squid-proxy-url"]
        if proxy_url:
            os.environ['HTTP_PROXY'] = proxy_url
            os.environ['HTTPS_PROXY'] = proxy_url

        # Fetch the code using git
        Repo.clone_from(self._stored.repo, APP_PATH, branch='vm-charm')

        try:
            check_output([str(CARGO_PATH), "build", "--release"], cwd=APP_PATH)
        except CalledProcessError as e:
            logger.error(f"Cargo build failed with error code {e.returncode}")
            self.unit.status = BlockedStatus("Cargo build failed")
            return

    def _on_database_created(self, _: DatabaseCreatedEvent):
        """Handle the database creation event."""
        logger.info("Database created event triggered.")
        self._setup_application()
        self._render_systemd_unit()
        self._start_ratings()

    def _start_ratings(self):
        """Start the ratings service using systemd."""
        logger.info("Attempting to start ratings service.")

        if self.model.get_relation("database") is None:
            logger.warning("No database relation found. Waiting.")
            self.unit.status = ops.WaitingStatus("Waiting for database relation")
            return

        try:
            logger.info("Resuming systemd service for ratings.")
            systemd.service_resume("ratings")
            self.unit.open_port(protocol="tcp", port=443)
            self.unit.status = ops.ActiveStatus()
            logger.info("Ratings service started successfully.")
        except Exception as e:
            logger.error(f"Failed to start ratings service: {str(e)}")
            self.unit.status = ops.BlockedStatus(f"Failed to start ratings service: {str(e)}")

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

    def _install_apt_packages(self, packages: list):
        """Simple wrapper around 'apt-get install -y"""
        try:
            apt.update()
            apt.add_package(packages)
        except apt.PackageNotFoundError:
            logger.error("a specified package not found in package cache or on system")
            self.unit.status = BlockedStatus("Failed to install packages")
        except apt.PackageError:
            logger.error("could not install package")
            self.unit.status = BlockedStatus("Failed to install packages")

    def _on_pull_and_rebuild(self, event):
        event.set_results({"status": "pulling and rebuilding"})
        try:
            # Pull new code
            repo = Repo(APP_PATH)
            repo.remotes.origin.pull()

            # Rebuild the application
            check_output([str(CARGO_PATH), "build", "--release"], cwd=APP_PATH)
            systemd.service_restart("ratings")

            event.set_results({"status": "successful"})
        except Exception as e:
            event.fail(f"Failed: {str(e)}")
            self.unit.status = BlockedStatus(f"Pull and rebuild failed: {str(e)}")


if __name__ == "__main__":  # pragma: nocover
    ops.main(RatingsCharm)
