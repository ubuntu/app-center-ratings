"""Control App Center Ratings on a host system. Provides a Ratings class."""

import logging

from charms.operator_libs_linux.v1 import snap

logger = logging.getLogger(__name__)


class Ratings:
    """Class representing Ratings on a host system."""

    def install(self):
        """Install the Ratings snap package."""
        try:
            self._snap.ensure(snap.SnapState.Latest, channel="edge")
            snap.hold_refresh()
        except snap.SnapError as e:
            logger.error("could not install ratings. Reason: %s", e.message)
            logger.debug(e, exc_info=True)
            raise e

    def refresh(self):
        """Refresh the Ratings snap if there is a new revision."""
        # The operation here is exactly the same, so just call the install method
        self.install()

    def start(self):
        """Start and enable Ratings using the snap service."""
        self._snap.start(enable=True)

    def stop(self):
        """Stop Ratings using the snap service."""
        self._snap.stop(disable=True)

    def remove(self):
        """Remove the Ratings snap."""
        self._snap.ensure(snap.SnapState.Absent)

    def configure(
        self,
        jwt_secret=None,
        log_level=None,
        postgres_uri=None,
        migration_postgres_uri=None,
        env=None,
    ):
        """Configure Ratings on the host system."""
        if jwt_secret:
            self._snap.set({"app-jwt-secret": jwt_secret})

        if log_level:
            self._snap.set({"app-log-level": log_level})

        if postgres_uri:
            self._snap.set({"app-postgres-uri": postgres_uri})

        if migration_postgres_uri:
            self._snap.set({"app-migration-postgres-uri": migration_postgres_uri})

        if env:
            self._snap.set({"app-env": env})

        # Restart the snap service
        self._snap.restart()

    @property
    def installed(self):
        """Report if the Ratings snap is installed."""
        return self._snap.present

    @property
    def running(self):
        """Report if the 'ratings-svc' snap service is running."""
        return self._snap.services["ratings-svc"]["active"]

    @property
    def _snap(self):
        """Return a representation of the Ratings snap."""
        cache = snap.SnapCache()
        return cache["ratings"]
