#!/usr/bin/env python3
# Copyright 2023 Canonical
# See LICENSE file for licensing details.

"""Ubuntu Software Centre ratings service.

A backend service to support application ratings in the new Ubuntu Software Centre.
"""

import logging

import ops

logger = logging.getLogger(__name__)


class RatingsCharm(ops.CharmBase):
    """Main operator class for ratings service."""

    def __init__(self, *args):
        super().__init__(*args)
        self.framework.observe(self.on.ratings_pebble_ready, self._on_ratings_pebble_ready)

    def _on_ratings_pebble_ready(self, event: ops.PebbleReadyEvent):
        """Define and start the workload using the Pebble API."""
        container = event.workload
        container.add_layer("ratings", self._pebble_layer, combine=True)
        container.replan()
        self.unit.status = ops.ActiveStatus()

    @property
    def _pebble_layer(self):
        """Return a dictionary representing a Pebble layer."""
        return {
            "summary": "ratings layer",
            "description": "pebble config layer for ratings",
            "services": {
                "ratings": {
                    "override": "replace",
                    "summary": "ratings",
                    "command": "/bin/ratings",
                    "startup": "enabled",
                    "environment": {
                        # TODO: Replace this placeholder
                        "POSTGRES": "postresql://user:password@localhost:5432/ratings",
                        # TODO: Replace this placeholder
                        "JWT_SECRET": "deadbeef",
                    },
                }
            },
        }


if __name__ == "__main__":  # pragma: nocover
    ops.main(RatingsCharm)
