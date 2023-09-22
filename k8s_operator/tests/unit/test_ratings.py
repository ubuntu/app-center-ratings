# Copyright 2023 Canonical
# See LICENSE file for licensing details.

import unittest
from unittest.mock import patch

from ratings import Ratings


class TestRatings(unittest.TestCase):
    def setUp(self):
        pass

    def test_ratings_constructor_props(self):
        r = Ratings("foobar", "deadbeef")
        self.assertEqual(r.connection_string, "foobar")
        self.assertEqual(r.jwt_secret, "deadbeef")

    @patch("database.RatingsDatabase.ready", lambda x: False)
    @patch("database.RatingsDatabase.create_tables", lambda x: True)
    def test_ratings_ready_db_not_initialised_jwt_not_present(self):
        r = Ratings("foobar", "")
        self.assertFalse(r.ready())

    @patch("database.RatingsDatabase.ready", lambda x: True)
    def test_ratings_ready_db_initialised_jwt_not_present(self):
        r = Ratings("foobar", "")
        self.assertFalse(r.ready())

    @patch("database.RatingsDatabase.ready", lambda x: True)
    def test_ratings_ready_db_initialised_jwt_present(self):
        r = Ratings("foobar", "deadbeef")
        self.assertTrue(r.ready())

    def test_ratings_pebble_layer(self):
        r = Ratings("foobar", "deadbeef")
        self.assertEqual(
            r.pebble_layer(),
            {
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
                            "APP_JWT_SECRET": "deadbeef",
                            "APP_POSTGRES_URI": "foobar",
                            "APP_LOG_LEVEL": "info",
                        },
                    }
                },
            },
        )
