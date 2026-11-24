# Copyright 2026 Canonical Ltd.
# See LICENSE file for licensing details.

"""Fixtures for the isolated live charm smoke test (not executed during collection)."""

from collections.abc import Iterator
from pathlib import Path

import jubilant
import pytest

from .helpers import (
    APP_NAME,
    DATABASE_NAME,
    READY_TIMEOUT,
    RatingsAPI,
    diagnostics,
    generated_api,
    port_forward,
)


@pytest.hookimpl(hookwrapper=True)
def pytest_runtest_makereport(item, call):
    """Record this test's outcome so teardown diagnostics also cover setup failures."""
    outcome = yield
    report = outcome.get_result()
    setattr(item, f"rep_{report.when}", report)


@pytest.fixture
def juju(request: pytest.FixtureRequest) -> Iterator[jubilant.Juju]:
    """Always create and destroy an ephemeral model; never use or switch a local model."""
    with jubilant.temp_model(cloud="microk8s", keep=False) as juju:
        juju.wait_timeout = READY_TIMEOUT
        try:
            yield juju
        finally:
            if any(
                getattr(request.node, f"rep_{phase}", None)
                and getattr(request.node, f"rep_{phase}").failed
                for phase in ("setup", "call")
            ):
                diagnostics(juju)


@pytest.fixture
def ratings_artifacts(
    charm_path: str, resource_images: dict[str, str]
) -> tuple[Path, dict[str, str]]:
    """Use only opcli's built charm and Ratings rock resource, with no fallback images."""
    charm = Path(charm_path).resolve(strict=True)
    if not charm.is_file() or charm.suffix != ".charm":
        raise ValueError("opcli must supply a built .charm file")
    image = resource_images.get("ratings-image")
    if not image:
        raise ValueError("opcli must supply the built ratings-image resource")
    if image.startswith(("/", "./", "../")):
        raise ValueError(
            "ratings-image is a local rock, not a registry reference; run "
            "'opcli artifacts push-images --missing-registry deploy' before testing"
        )
    return charm, {"ratings-image": image}


@pytest.fixture
def deployed(ratings_artifacts, juju: jubilant.Juju) -> jubilant.Juju:
    """Deploy one Ratings unit and its own one-unit PostgreSQL, then wait for readiness."""
    charm, resources = ratings_artifacts
    juju.deploy(
        "postgresql-k8s",
        app=DATABASE_NAME,
        channel="16/stable",
        num_units=1,
        trust=True,
        config={"profile": "testing"},
    )
    juju.deploy(charm, app=APP_NAME, resources=resources, num_units=1)
    juju.integrate(f"{APP_NAME}:database", f"{DATABASE_NAME}:database")
    juju.wait(
        lambda status: (
            jubilant.all_active(status, APP_NAME, DATABASE_NAME)
            and jubilant.all_agents_idle(status, APP_NAME, DATABASE_NAME)
        ),
        error=jubilant.any_error,
        timeout=READY_TIMEOUT,
        delay=5,
    )
    return juju


@pytest.fixture(scope="session")
def ratings_api() -> Iterator[RatingsAPI]:
    """Generate real gRPC clients in a temporary directory, only when requested."""
    with generated_api() as api:
        yield api


@pytest.fixture
def grpc_target(deployed: jubilant.Juju) -> Iterator[str]:
    """Reach the real workload over an ephemeral, loopback-only MicroK8s port-forward."""
    with port_forward(deployed.model) as target:
        yield target
