#!/usr/bin/env python3
# Copyright 2023 Canonical
# See LICENSE file for licensing details.

import asyncio
import json
import logging
import os
import secrets
from pathlib import Path

import grpc
import pytest
import ratings_api.ratings_features_user_pb2 as pb2
import ratings_api.ratings_features_user_pb2_grpc as pb2_grpc
import yaml
from pytest import mark
from pytest_operator.plugin import OpsTest

logger = logging.getLogger(__name__)

METADATA = yaml.safe_load(Path("./metadata.yaml").read_text())
RATINGS = "ratings"
DB = "db"
TRAEFIK = "traefik"


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
        ops_test.model.deploy(charm, resources=resources, application_name=RATINGS, trust=True),
        ops_test.model.wait_for_idle(
            apps=[RATINGS], status="waiting", raise_on_blocked=True, timeout=1000
        ),
    )


@mark.abort_on_fail
async def test_database_relation(ops_test: OpsTest):
    """Test that the charm can be successfully related to PostgreSQL."""
    await asyncio.gather(
        ops_test.model.deploy(
            "postgresql-k8s", channel="14/edge", application_name=DB, trust=True
        ),
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
async def test_ratings_scale(ops_test: OpsTest):
    """Test that the charm can be scaled out and still talk to the database."""
    await asyncio.gather(
        ops_test.model.applications[RATINGS].scale(2),
        ops_test.model.wait_for_idle(
            apps=[RATINGS],
            status="active",
            timeout=1000,
            wait_for_exact_units=2,
        ),
    )


@mark.abort_on_fail
async def test_ratings_register_user(ops_test: OpsTest):
    """End-to-end test to ensure the app can interact with the database."""
    status = await ops_test.model.get_status()  # noqa: F821
    address = status["applications"][RATINGS]["public-address"]

    channel = grpc.insecure_channel(f"{address}:18080")
    stub = pb2_grpc.UserStub(channel)
    message = pb2.RegisterRequest(id=secrets.token_hex(32))
    response = stub.Register(message)
    assert response.token


@pytest.mark.abort_on_fail
async def test_ingress_traefik_k8s(ops_test):
    """Test that the charm can be integrated with the Traefik ingress."""
    await asyncio.gather(
        ops_test.model.deploy(
            "traefik-k8s",
            application_name=TRAEFIK,
            channel="edge",
            config={"routing_mode": "subdomain", "external_hostname": "foo.bar"},
            trust=True,
        ),
        ops_test.model.wait_for_idle(apps=[TRAEFIK], status="active", timeout=1000),
    )

    # Create the relation
    await ops_test.model.integrate(f"{RATINGS}:ingress", TRAEFIK)
    # Wait for the two apps to quiesce
    await ops_test.model.wait_for_idle(apps=[RATINGS, TRAEFIK], status="active", timeout=1000)

    result = await _retrieve_proxied_endpoints(ops_test, TRAEFIK)
    assert result.get(RATINGS, None) == {"url": f"http://{ops_test.model_name}-{RATINGS}.foo.bar/"}


@pytest.mark.skipif(
    (not os.environ.get("GITHUB_ACTION", "")),
    reason="""This test requires host configuration which might not be present on your machine.

    If you know what you're doing, run `export GITHUB_ACTION=Foo` or similar prior to running
    `tox -e integration`.
    """,
)
async def test_ratings_register_user_through_ingress(ops_test: OpsTest):
    """End-to-end test to ensure the app can be interacted with from behind Traefik."""
    address = f"{ops_test.model_name}-{RATINGS}.foo.bar:80"
    channel = grpc.insecure_channel(address)
    stub = pb2_grpc.UserStub(channel)
    message = pb2.RegisterRequest(id=secrets.token_hex(32))
    response = stub.Register(message)
    assert response.token


async def _retrieve_proxied_endpoints(ops_test, traefik_application_name):
    traefik_application = ops_test.model.applications[traefik_application_name]
    traefik_first_unit = next(iter(traefik_application.units))
    action = await traefik_first_unit.run_action("show-proxied-endpoints")
    await action.wait()
    result = await ops_test.model.get_action_output(action.id)

    return json.loads(result["proxied-endpoints"])
