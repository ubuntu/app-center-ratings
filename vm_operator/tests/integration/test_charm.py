#!/usr/bin/env python3
# Copyright 2022 Canonical Ltd.
# See LICENSE file for licensing details.

import asyncio
import secrets

import grpc
import ratings_api.ratings_features_user_pb2 as pb2
import ratings_api.ratings_features_user_pb2_grpc as pb2_grpc
from pytest import mark
from pytest_operator.plugin import OpsTest

RATINGS = "ratings"
UNIT_0 = f"{RATINGS}/0"
DB = "db"


@mark.abort_on_fail
@mark.skip_if_deployed
async def test_deploy(ops_test: OpsTest, ratings_charm):
    await ops_test.model.deploy(await ratings_charm, application_name=RATINGS)
    # issuing dummy update_status just to trigger an event
    async with ops_test.fast_forward():
        await ops_test.model.wait_for_idle(apps=[RATINGS], status="active", timeout=1000)
        assert ops_test.model.applications[RATINGS].units[0].workload_status == "active"


@mark.abort_on_fail
async def test_database_relation(ops_test: OpsTest):
    """Test that the charm can be successfully related to PostgreSQL."""
    await asyncio.gather(
        ops_test.model.deploy("postgresql", channel="edge", application_name=DB, trust=True),
        ops_test.model.wait_for_idle(
            apps=[DB], status="active", raise_on_blocked=True, timeout=1000
        ),
    )

    await asyncio.gather(
        ops_test.model.integrate(RATINGS, DB),
        ops_test.model.wait_for_idle(
            apps=[RATINGS], status="active", raise_on_blocked=True, timeout=1000
        ),
    )


@mark.abort_on_fail
async def test_ratings_register_user(ops_test: OpsTest):
    """End-to-end test to ensure the app can interact with the database."""
    status = await ops_test.model.get_status()  # noqa: F821
    unit = list(status.applications[RATINGS].units)[0]
    print(f"Connecting to address: {status}")
    address = status["applications"][RATINGS]["units"][unit]["public-address"]
    print(f"Connecting to address: {address}")
    connection_string = f"{address}:443"

    channel = grpc.insecure_channel(connection_string)
    stub = pb2_grpc.UserStub(channel)
    message = pb2.AuthenticateRequest(id=secrets.token_hex(32))
    print(f"Message sent: {message}")
    response = stub.Authenticate(message)
    assert response.token
