#!/usr/bin/env python3
# Copyright 2023 Canonical
# See LICENSE file for licensing details.

import asyncio
import logging
from pathlib import Path

import pytest
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
