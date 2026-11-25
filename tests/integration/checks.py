# Copyright 2026 Canonical Ltd.
# See LICENSE file for licensing details.

"""Offline helper checks: invoke this file explicitly; spread only discovers test_*.py."""

import configparser
import importlib
import importlib.metadata
import json
import subprocess
import sys
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import MagicMock, call, patch

import grpc
import jubilant
import pytest
from opcli import pytest_plugin
from opcli.core import env, provision, spread, yaml_io
from opcli.models.artifacts_build import ArtifactsGenerated

from . import conftest, helpers

ROOT = Path(__file__).resolve().parents[2]
OPCLI_SHA = "14b1cf49d48b9d5b96192d9bca2890c98b7295cf"


def test_generated_clients_and_import_cleanup():
    """Check real protobuf generation and gRPC stub construction without a connection."""
    with helpers.generated_api() as api:
        request = api.user_pb.AuthenticateRequest(id="a" * 64)
        assert api.user_pb.AuthenticateRequest.FromString(request.SerializeToString()) == request
        bulk = api.app_pb.GetBulkRatingsRequest(snap_ids=["unknown"])
        assert list(bulk.snap_ids) == ["unknown"]
        api.user_grpc.UserStub(MagicMock())
        api.app_grpc.AppStub(MagicMock())
        generated_dir = Path(api.user_pb.__file__).parent
        assert generated_dir.is_dir()
    assert not generated_dir.exists()
    assert "ratings_features_user_pb2" not in sys.modules
    assert "ratings_features_common_pb2" not in sys.modules


@pytest.mark.parametrize("model", [None, "default", "staging", "controller:admin/local"])
def test_reject_non_temporary_namespace(model):
    with pytest.raises(ValueError, match="fresh Jubilant temporary model"):
        helpers.kubectl(model, "get", "pods")


def test_port_forward_readiness_and_cleanup(monkeypatch):
    process = MagicMock()
    process.poll.return_value = None
    popen = MagicMock(return_value=process)
    selector = MagicMock()
    selector.__enter__.return_value = selector
    selector.select.return_value = [(object(), 1)]
    monkeypatch.setattr(helpers.subprocess, "Popen", popen)
    monkeypatch.setattr(helpers.selectors, "DefaultSelector", lambda: selector)
    monkeypatch.setattr(
        helpers.os, "read", lambda *args: b"Forwarding from 127.0.0.1:43210 -> 8080\n"
    )
    with pytest.raises(AssertionError, match="test failed"):
        with helpers.port_forward("controller:admin/jubilant-abcd1234") as target:
            assert target == "127.0.0.1:43210"
            raise AssertionError("test failed")
    command = popen.call_args.args[0]
    assert command == [
        "microk8s",
        "kubectl",
        "--namespace",
        "jubilant-abcd1234",
        "port-forward",
        "service/ratings",
        ":8080",
        "--address=127.0.0.1",
        "--pod-running-timeout=30s",
    ]
    process.terminate.assert_called_once()
    process.wait.assert_called_once_with(timeout=5)
    process.stdout.close.assert_called_once()


def test_port_forward_startup_timeout_kills_and_reaps(monkeypatch):
    process = MagicMock()
    process.poll.return_value = None
    process.wait.side_effect = [subprocess.TimeoutExpired("port-forward", 5), 0]
    monkeypatch.setattr(helpers.subprocess, "Popen", lambda *args, **kwargs: process)
    monkeypatch.setattr(helpers.selectors, "DefaultSelector", MagicMock())
    monkeypatch.setattr(helpers.time, "monotonic", MagicMock(side_effect=[0, 46]))
    with pytest.raises(TimeoutError, match="45 seconds"):
        with helpers.port_forward("jubilant-abcd1234"):
            pytest.fail("A port-forward without readiness must not yield")
    process.terminate.assert_called_once()
    process.kill.assert_called_once()
    assert process.wait.call_args_list == [call(timeout=5), call(timeout=5)]


@pytest.mark.parametrize("has_ratings", [False, True])
def test_grpc_contract_and_delete_on_failed_read(monkeypatch, has_ratings):
    channel = MagicMock()
    channel.__enter__.return_value = channel
    monkeypatch.setattr(helpers.grpc, "insecure_channel", lambda target: channel)
    ready = MagicMock()
    monkeypatch.setattr(helpers.grpc, "channel_ready_future", lambda channel: ready)
    user, app = MagicMock(), MagicMock()
    user.Authenticate.return_value.token = "fake-token-for-offline-test"
    app.GetBulkRatings.return_value.ratings = [object()] if has_ratings else []
    api = SimpleNamespace(
        user_grpc=SimpleNamespace(UserStub=lambda channel: user),
        app_grpc=SimpleNamespace(AppStub=lambda channel: app),
        user_pb=SimpleNamespace(AuthenticateRequest=lambda **kwargs: kwargs),
        app_pb=SimpleNamespace(GetBulkRatingsRequest=lambda **kwargs: kwargs),
    )
    if has_ratings:
        with pytest.raises(AssertionError, match="unexpectedly returned ratings"):
            helpers.smoke_rpcs(api, "127.0.0.1:43210", "a" * 64)
    else:
        helpers.smoke_rpcs(api, "127.0.0.1:43210", "a" * 64)
    ready.result.assert_called_once_with(timeout=30)
    user.Authenticate.assert_called_once_with({"id": "a" * 64}, timeout=10)
    app.GetBulkRatings.assert_called_once_with(
        {"snap_ids": ["a" * 64]},
        timeout=10,
        metadata=(("authorization", "Bearer fake-token-for-offline-test"),),
    )
    user.Delete.assert_called_once()
    assert user.Delete.call_args.kwargs == {
        "timeout": 10,
        "metadata": (("authorization", "Bearer fake-token-for-offline-test"),),
    }


def test_rpc_error_hides_details():
    class SensitiveError(grpc.RpcError):
        def code(self):
            return grpc.StatusCode.UNAVAILABLE

        def __str__(self):
            return "private database credentials and JWT"

    with pytest.raises(AssertionError) as error:
        helpers.rpc(MagicMock(side_effect=SensitiveError()), object())
    assert str(error.value) == "Ratings RPC failed: UNAVAILABLE"
    assert error.value.__suppress_context__


def test_deployment_contract():
    juju = MagicMock()
    charm = Path("/unused/mock.charm")
    resources = {"ratings-image": "localhost:32000/ratings:test"}
    result = conftest.deployed.__wrapped__((charm, resources), juju)
    assert result is juju
    assert juju.deploy.call_args_list == [
        call(
            "postgresql-k8s",
            app="postgresql",
            channel="16/stable",
            num_units=1,
            trust=True,
            config={"profile": "testing"},
        ),
        call(charm, app="ratings", resources=resources, num_units=1),
    ]
    juju.integrate.assert_called_once_with("ratings:database", "postgresql:database")
    assert juju.wait.call_args.kwargs == {
        "error": jubilant.any_error,
        "timeout": 900,
        "delay": 5,
    }


@pytest.mark.parametrize("failed", [False, True])
def test_temp_model_is_always_cleaned_up(monkeypatch, failed):
    temporary = MagicMock()
    juju = temporary.__enter__.return_value
    factory = MagicMock(return_value=temporary)
    monkeypatch.setattr(conftest.jubilant, "temp_model", factory)
    diagnostic = MagicMock()
    monkeypatch.setattr(conftest, "diagnostics", diagnostic)
    request = SimpleNamespace(node=SimpleNamespace(rep_setup=SimpleNamespace(failed=failed)))
    fixture = conftest.juju.__wrapped__(request)
    assert next(fixture) is juju
    fixture.close()
    factory.assert_called_once_with(cloud="microk8s", keep=False)
    temporary.__exit__.assert_called_once()
    assert juju.wait_timeout == 900
    assert diagnostic.call_args_list == ([call(juju)] if failed else [])


def test_diagnostics_are_bounded_and_exclude_secrets(monkeypatch, capsys):
    juju_data = {
        "applications": {
            "ratings": {
                "application-status": {"current": "active", "message": "private-token"},
                "units": {"ratings/0": {"workload-status": {"current": "active"}}},
                "relation-data": "private-token",
            }
        }
    }
    pods_data = {
        "items": [
            {
                "metadata": {"name": "ratings-0"},
                "spec": {"env": "private-token"},
                "status": {"phase": "Running"},
            }
        ]
    }
    run = MagicMock(
        side_effect=[
            SimpleNamespace(stdout=json.dumps(juju_data)),
            SimpleNamespace(stdout=json.dumps(pods_data)),
        ]
    )
    monkeypatch.setattr(helpers.subprocess, "run", run)
    helpers.diagnostics(jubilant.Juju(model="jubilant-abcd1234"))
    output = capsys.readouterr().out
    assert "private-token" not in output
    assert "active" in output and "Running" in output
    assert all(command.kwargs["timeout"] == 20 for command in run.call_args_list)


@pytest.mark.parametrize("image", ["/tmp/ratings.rock", "./ratings.rock", "../ratings.rock"])
def test_file_resource_is_rejected(tmp_path, image):
    charm = tmp_path / "ratings.charm"
    charm.write_bytes(b"offline charm placeholder")
    with pytest.raises(ValueError, match="opcli artifacts push-images"):
        conftest.ratings_artifacts.__wrapped__(str(charm), {"ratings-image": image})


@pytest.fixture
def artifact_plan():
    """Read the committed build plan, not a fabricated replacement for root config."""
    return yaml_io.load_artifacts_plan(ROOT / "artifacts.yaml")


@pytest.fixture
def localized_manifest(tmp_path, artifact_plan, monkeypatch):
    """Model fetched CI files; fake archives are never unpacked or deployed."""
    charm = tmp_path / "app-center-ratings_amd64.charm"
    charm.write_bytes(b"offline charm placeholder")
    rock = tmp_path / "ratings_amd64.rock"
    rock.write_bytes(b"offline rock placeholder")
    generated = ArtifactsGenerated.model_validate(
        {
            "version": 1,
            "rocks": [
                {
                    "name": artifact_plan.rocks[0].name,
                    "rockcraft-yaml": artifact_plan.rocks[0].rockcraft_yaml,
                    "builds": [{"arch": "amd64", "file": rock.name}],
                }
            ],
            "charms": [
                {
                    "name": artifact_plan.charms[0].name,
                    "charmcraft-yaml": artifact_plan.charms[0].charmcraft_yaml,
                    "resources": {
                        name: resource.model_dump(exclude_none=True)
                        for name, resource in artifact_plan.charms[0].resources.items()
                    },
                    "builds": [{"arch": "amd64", "base": "ubuntu-24.04", "path": charm.name}],
                }
            ],
        }
    )
    manifest = tmp_path / "build" / "artifacts.build.yaml"
    yaml_io.dump_artifacts_build(generated, manifest)
    monkeypatch.setenv("OPCLI_ARTIFACTS_BUILD_YAML", str(manifest))
    monkeypatch.setattr(env, "current_arch", lambda: "amd64")
    return manifest


def plugin_request():
    """Only artifact discovery options are read; no live runtime fixtures are requested."""
    return SimpleNamespace(config=SimpleNamespace(getoption=lambda *args, **kwargs: None))


def test_root_artifact_resource_mapping(artifact_plan):
    assert len(artifact_plan.rocks) == len(artifact_plan.charms) == 1
    assert not artifact_plan.snaps
    rock, charm = artifact_plan.rocks[0], artifact_plan.charms[0]
    assert rock.name == "ratings"
    assert rock.rockcraft_yaml == "ratings_rock/rockcraft.yaml"
    assert (ROOT / (rock.pack_dir or Path(rock.rockcraft_yaml).parent)).resolve() == (
        ROOT / "ratings_rock"
    ).resolve()
    assert charm.name == "app-center-ratings"
    assert charm.charmcraft_yaml == "charmcraft.yaml"
    assert list(charm.resources) == ["ratings-image"]
    assert charm.resources["ratings-image"].type == "oci-image"
    assert charm.resources["ratings-image"].rock == rock.name
    assert [target.arch for target in rock.platforms] == ["amd64"]
    assert [target.arch for target in charm.platforms] == ["amd64"]
    metadata = yaml_io.load_yaml(ROOT / charm.charmcraft_yaml)
    assert metadata["name"] == charm.name
    assert metadata["containers"]["ratings"]["resource"] == "ratings-image"
    assert metadata["resources"]["ratings-image"]["type"] == "oci-image"


def test_unpushed_rock_is_rejected_before_deploy(localized_manifest):
    request = plugin_request()
    charm = pytest_plugin.charm_path.__wrapped__(request)
    resources = pytest_plugin.resource_images.__wrapped__(request)
    assert resources == {"ratings-image": str(localized_manifest.parents[1] / "ratings_amd64.rock")}
    with pytest.raises(ValueError, match="opcli artifacts push-images"):
        conftest.ratings_artifacts.__wrapped__(charm, resources)


def test_registry_push_updates_plugin_resource(localized_manifest, monkeypatch):
    """Exercise real manifest rewriting with skopeo and registry access fully mocked."""
    run = MagicMock()
    monkeypatch.setattr(provision, "run_command", run)
    registry = MagicMock(return_value=True)
    monkeypatch.setattr(provision, "_ensure_registry_available", registry)
    monkeypatch.setattr(provision, "_skopeo_binary", lambda: "skopeo")
    root = localized_manifest.parents[1]
    pushed = provision.provision_load(root, missing_registry="deploy")
    registry.assert_called_once_with(root, "localhost:32000", "deploy")
    assert len(pushed) == 1 and pushed[0].startswith("localhost:32000/ratings:amd64-")
    run.assert_called_once_with(
        [
            "sudo",
            "skopeo",
            "--insecure-policy",
            "copy",
            "--dest-tls-verify=false",
            "oci-archive:ratings_amd64.rock",
            f"docker://{pushed[0]}",
        ],
        cwd=str(root),
    )
    request = plugin_request()
    resources = pytest_plugin.resource_images.__wrapped__(request)
    assert resources == {"ratings-image": pushed[0]}
    charm, checked_resources = conftest.ratings_artifacts.__wrapped__(
        pytest_plugin.charm_path.__wrapped__(request), resources
    )
    assert charm == root / "app-center-ratings_amd64.charm"
    assert checked_resources == resources
    assert yaml_io.load_artifacts_build(localized_manifest).rocks[0].builds[0].file


def test_opcli_microk8s_selection_is_bounded(monkeypatch):
    monkeypatch.setattr(
        provision.shutil, "which", lambda name: name if name == "microk8s" else None
    )
    run = MagicMock()
    monkeypatch.setattr(provision, "run_command", run)
    assert provision._detect_kubectl() == ["sudo", "microk8s", "kubectl"]
    run.assert_called_once_with(
        ["sudo", "microk8s", "kubectl", "cluster-info", "--request-timeout=5s"],
        stream=False,
        quiet=True,
        timeout=10,
    )


def test_spread_discovers_only_live_smoke_module():
    expanded = yaml_io.loads_yaml(spread.spread_expand(ROOT, ci=True))
    suite = expanded["suites"]["build/tests/integration/"]
    assert suite["backends"] == ["integration-test-ci"]
    assert {
        key: value for key, value in suite["environment"].items() if key.startswith("MODULE/")
    } == {"MODULE/test_smoke": "tests/integration/test_smoke.py"}
    assert Path(suite["environment"]["OPCLI_CWD"]) == Path(".")
    backend = expanded["backends"]["integration-test-ci"]
    assert [next(iter(system)) for system in backend["systems"]] == ["ubuntu-24.04"]
    prepare = backend["prepare"]
    assert prepare.index("opcli artifacts fetch") < prepare.index("opcli artifacts push-images")
    assert "opcli artifacts push-images --missing-registry deploy" in prepare
    assert '--module "${MODULE}"' in spread._TASK_YAML_CONTENT_SUITE


def test_opcli_module_replaces_tox_default():
    """Test real argument assembly without installing the CLI-only Jinja2 dependency."""
    # This suite has no templates. Mock only the unused template-rendering imports;
    # bare opcli deliberately excludes Jinja2, which the separate opcli CLI installs.
    render = MagicMock(side_effect=AssertionError("This suite must not render templates"))
    template = SimpleNamespace(render_arguments_template=render, render_environment_template=render)
    with patch.dict(sys.modules, {"opcli.core.template": template}):
        module = importlib.import_module("opcli.core.pytest_args")
        try:
            config = spread.get_suite_config(ROOT, "tests/integration/")
            assert not config.get("pytest-arguments-template")
            assert not config.get("pytest-environment-template")
            assert module.assemble_tox_argv(ROOT, suite_config=config) == [
                "tox",
                "-e",
                "integration",
            ]
            argv = module.assemble_tox_argv(
                ROOT, suite_config=config, module_path="tests/integration/test_smoke.py"
            )
            assert argv == ["tox", "-e", "integration", "--", "tests/integration/test_smoke.py"]
            render.assert_not_called()
        finally:
            sys.modules.pop("opcli.core.pytest_args", None)
    tox = configparser.ConfigParser(interpolation=None)
    tox.read(ROOT / "tox.ini")
    command = tox["testenv:integration"]["commands"].strip()
    assert command.split() == [
        "python",
        "-m",
        "pytest",
        "-v",
        "--tb=short",
        "{posargs:tests/integration}",
    ]
    assert (
        command.replace("{posargs:tests/integration}", argv[-1])
        .split()
        .count("tests/integration/test_smoke.py")
        == 1
    )
    assert {"HOME", "JUJU_DATA", "OPCLI_ARTIFACTS_BUILD_YAML"} <= set(
        tox["testenv:integration"]
        .get("pass_env", tox["testenv:integration"].get("passenv", ""))
        .replace(",", " ")
        .split()
    )


def test_workflow_uses_pinned_artifacts_not_publication():
    workflow = yaml_io.load_yaml(ROOT / ".github" / "workflows" / "integration_test.yaml")
    jobs = workflow["jobs"]
    uses = f"canonical/charm-ci/.github/workflows/integration-test.yml@{OPCLI_SHA}"
    callers = [job for job in jobs.values() if job.get("uses") == uses]
    assert len(callers) == 1
    caller = callers[0]
    assert caller["with"]["upload-image"] == "artifact"
    assert caller["with"].get("working-directory", ".") == "."
    assert not any("publish" in job.get("uses", "") for job in jobs.values())
    assert not any(key.startswith("test-secret-") for key in caller.get("with", {}))
    # Upstream's nested build job still requests packages:write in artifact mode.
    permissions = caller.get("permissions", workflow.get("permissions", {}))
    assert permissions["packages"] == "write"
    assert permissions["contents"] == permissions["actions"] == "read"
    assert f"charm-ci.git@{OPCLI_SHA}" in (ROOT / "requirements-integration.txt").read_text()
    distribution = importlib.metadata.distribution("opcli")
    assert distribution.version == "1.0.2"
    direct_url = json.loads(distribution.read_text("direct_url.json"))
    assert direct_url["vcs_info"]["commit_id"] == OPCLI_SHA
