#!/usr/bin/env python3
# Copyright 2023 Canonical
# See LICENSE file for licensing details.

import asyncio
import logging
from pathlib import Path

import grpc
import pytest
import ratings_features_user_pb2 as pb2
import ratings_features_user_pb2_grpc as pb2_grpc
import yaml
from pytest import mark
from pytest_operator.plugin import OpsTest

logger = logging.getLogger(__name__)

METADATA = yaml.safe_load(Path("./metadata.yaml").read_text())
RATINGS = "ratings"
DB = "db"


@pytest.mark.abort_on_fail
async def test_build_and_deploy(ops_test: OpsTest):
    """Build the charm-under-test and deploy it together with related charms.

    Assert on the unit status before any relations/configurations take place.
    """
    # Build and deploy charm from local source folder
    charm = await ops_test.build_charm(".")
    resources = {"ratings-image": METADATA["resources"]["ratings-image"]["upstream-source"]}

    # Deploy the charm and wait for active/idle status
    await asyncio.gather(
        ops_test.model.deploy(charm, resources=resources, application_name=RATINGS),
        ops_test.model.wait_for_idle(
            apps=[RATINGS], status="waiting", raise_on_blocked=True, timeout=1000
        ),
    )


@mark.abort_on_fail
async def test_database_relation(ops_test: OpsTest):
    await asyncio.gather(
        ops_test.model.deploy("postgresql-k8s", channel="14/edge", application_name=DB),
        ops_test.model.wait_for_idle(
            apps=[DB], status="active", raise_on_blocked=True, timeout=1000
        ),
    )

    await asyncio.gather(
        ops_test.model.relate(RATINGS, DB),
        ops_test.model.wait_for_idle(
            apps=[RATINGS], status="active", raise_on_blocked=True, timeout=1000
        ),
    )


@mark.abort_on_fail
async def test_ratings_scale(ops_test: OpsTest):
    await asyncio.gather(
        await ops_test.model.applications[RATINGS].scale(2),
        await ops_test.model.wait_for_idle(
            apps=[RATINGS],
            status="active",
            timeout=1000,
            wait_for_exact_units=2,
        ),
    )

@mark.abort_on_fail
async def test_ratings_register_user(ops_test: OpsTest):
    status = await ops_test.model.get_status()  # noqa: F821
    address = status["applications"][RATINGS]["public-address"]

    channel = grpc.insecure_channel(f"{address}:18080")
    stub = pb2_grpc.UserStub(channel)
    message = pb2.RegisterRequest(id="7060d63f5660924e55fd7e88cbb2046e15e80ed56aa463af57f2741d9f7c98cb")
    response = stub.Register(message)
    assert response.token
