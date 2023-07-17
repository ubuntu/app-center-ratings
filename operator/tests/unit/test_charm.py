# Copyright 2023 Canonical
# See LICENSE file for licensing details.

import unittest

import ops
import ops.testing
from charm import RatingsCharm


class TestCharm(unittest.TestCase):
    def setUp(self):
        self.harness = ops.testing.Harness(RatingsCharm)
        self.addCleanup(self.harness.cleanup)
        self.harness.begin()

    def test_ratings_pebble_ready(self):
        # Expected plan after Pebble ready with default config
        expected_plan = {
            "services": {
                "ratings": {
                    "override": "replace",
                    "summary": "ratings",
                    "command": "/bin/ratings",
                    "startup": "enabled",
                    "environment": {
                        "POSTGRES": "postresql://user:password@localhost:5432/ratings",
                        "JWT_SECRET": "deadbeef",
                    },
                }
            },
        }
        # Simulate the container coming up and emission of pebble-ready event
        self.harness.container_pebble_ready("ratings")
        # Get the plan now we've run PebbleReady
        updated_plan = self.harness.get_container_pebble_plan("ratings").to_dict()
        # Check we've got the plan we expected
        self.assertEqual(expected_plan, updated_plan)
        # Check the service was started
        service = self.harness.model.unit.get_container("ratings").get_service("ratings")
        self.assertTrue(service.is_running())
        # Ensure we set an ActiveStatus with no message
        self.assertEqual(self.harness.model.unit.status, ops.ActiveStatus())
