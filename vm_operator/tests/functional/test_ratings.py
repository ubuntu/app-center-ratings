import unittest
from pathlib import Path
from unittest import mock

from ratings import Ratings


class TestRatings(unittest.TestCase):
    def setUp(self):
        self.ratings = Ratings()
        if not self.ratings.installed:
            self.ratings.install()

    def test_install(self):
        self.assertTrue(Path("/snap/ratings/current/bin/ratings").exists())
        self.assertTrue(self.ratings.installed)

    def test_start(self):
        self.ratings.start()
        self.assertTrue(self.ratings.running)
        self.ratings.remove()

    def test_stop(self):
        self.ratings.stop()
        self.assertFalse(self.ratings.running)
        self.ratings.remove()

    def test_remove(self):
        self.ratings.remove()
        self.assertFalse(self.ratings.installed)

    @mock.patch("charms.operator_libs_linux.v1.snap.Snap.restart")
    def test_configure_ratings(self, _restart):
        self.ratings.configure(
            jwt_secret="foo",
            log_level="bar",
            postgres_uri="foobar",
            migration_postgres_uri="barfoo",
            env="testenv",
        )

        # Test have been updated
        self.assertEqual(self.ratings._snap.get("app-jwt-secret"), "foo")
        self.assertEqual(self.ratings._snap.get("app-log-level"), "bar")
        self.assertEqual(self.ratings._snap.get("app-migration-postgres-uri"), "barfoo")
        self.assertEqual(self.ratings._snap.get("app-postgres-uri"), "foobar")
        self.assertEqual(self.ratings._snap.get("app-env"), "testenv")

        _restart.assert_called_once()
