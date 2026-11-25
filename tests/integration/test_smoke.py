# Copyright 2026 Canonical Ltd.
# See LICENSE file for licensing details.

"""Real charm, rock, PostgreSQL relation, migrations, and authenticated gRPC smoke test."""

import hashlib
import secrets

from .helpers import RatingsAPI, smoke_rpcs


def test_deploy_and_grpc(ratings_api: RatingsAPI, grpc_target: str) -> None:
    """Authenticate, read an unknown snap ID, and delete the temporary database identity."""
    identifier = hashlib.sha256(secrets.token_bytes(32)).hexdigest()
    smoke_rpcs(ratings_api, grpc_target, identifier)
