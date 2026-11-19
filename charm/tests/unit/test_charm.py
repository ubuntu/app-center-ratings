"""Offline lifecycle tests using real Canonical libraries and Juju/Pebble Harness fakes."""

import base64
import json
from pathlib import Path
from unittest.mock import MagicMock, call, patch
from urllib.parse import parse_qs, unquote, urlsplit

import ops
import pytest
import yaml
from charms.data_platform_libs.v0.data_interfaces import DatabaseRequires
from charms.traefik_k8s.v0.traefik_route import TraefikRouteRequirer
from ops import pebble
from ops.testing import ActionFailed, Harness

from charm import (
    CA_PATH,
    DATABASE,
    INGRESS,
    JWT_GENERATION,
    JWT_ID,
    JWT_LABEL,
    PEERS,
    PORT,
    READY_CHECK,
    SERVICE,
    ConfigurationError,
    RatingsCharm,
    postgres_uri,
    validate_external_hostname,
)

METADATA = yaml.safe_load((Path(__file__).parents[2] / "charmcraft.yaml").read_text())
DB_DATA = {
    "database": DATABASE,
    "username": "relation-user",
    "password": "db-pass:@/?#% +",
    "endpoints": "postgresql-k8s-primary.ratings-dev.svc.cluster.local:5432",
    "tls": "False",
}


@pytest.fixture
def harness(monkeypatch, request):
    monkeypatch.setenv("JUJU_VERSION", "3.6.29")
    monkeypatch.setattr(
        "charm.socket.create_connection", MagicMock(side_effect=ConnectionRefusedError)
    )
    h = Harness(
        RatingsCharm,
        meta=yaml.safe_dump(METADATA),
        config=yaml.safe_dump(METADATA["config"]),
        actions=yaml.safe_dump(METADATA["actions"]),
    )
    h.set_leader(True)
    h.update_config(getattr(request, "param", {}))
    h.begin()
    h.set_can_connect(SERVICE, True)
    h.charm.container.add_layer(
        "image",
        {
            "services": {
                SERVICE: {"override": "replace", "startup": "enabled", "command": "/bin/ratings"}
            }
        },
    )
    h.add_relation(PEERS, h.model.app.name)
    yield h
    h.cleanup()


def database(h, data=None, *, secret=False):
    relation_id = h.add_relation("database", "postgresql-k8s")
    h.add_relation_unit(relation_id, "postgresql-k8s/0")
    values = dict(DB_DATA if data is None else data)
    if secret:
        secret_id = h.add_model_secret(
            "postgresql-k8s", {key: values.pop(key) for key in ("username", "password")}
        )
        h.grant_secret(secret_id, h.model.app)
        values["secret-user"] = secret_id
    h.update_relation_data(relation_id, "postgresql-k8s", values)
    return relation_id


def environment(h):
    return h.get_container_pebble_plan(SERVICE).services[SERVICE].environment


def mark_ready(h):
    # Harness does not execute checks; mock only their result, not the charm's health logic.
    info = pebble.CheckInfo(
        READY_CHECK, pebble.CheckLevel.READY, pebble.CheckStatus.UP, successes=1, threshold=1
    )
    with patch.object(ops.Container, "get_checks", return_value={READY_CHECK: info}):
        h.charm.on.update_status.emit()


def assert_stopped(h):
    assert not h.charm.container.get_service(SERVICE).is_running()
    plan = h.get_container_pebble_plan(SERVICE)
    assert plan.services[SERVICE].startup == "disabled"
    assert plan.services[SERVICE].environment == {}
    assert plan.checks[READY_CHECK].startup == pebble.CheckStartup.DISABLED
    assert not h.model.unit.opened_ports()


def test_official_database_request_is_dedicated_unprivileged(harness):
    relation_id = database(harness)
    assert isinstance(harness.charm.database, DatabaseRequires)
    data = harness.get_relation_data(relation_id, harness.model.app.name)
    assert data["database"] == "ratings"
    assert "extra-user-roles" not in data
    assert "username" in json.loads(data["requested-secrets"])
    assert "password" not in harness.get_relation_data(
        harness.model.get_relation(PEERS).id, harness.model.app.name
    )


@pytest.mark.parametrize("name", ["stg-app-center-rating", "prod-app-center-rating", "a" * 63])
def test_configured_sql_database_is_requested_and_used(harness, name):
    harness.update_config({"database-name": name})
    relation_id = database(harness, DB_DATA | {"database": name})
    assert harness.get_relation_data(relation_id, harness.model.app.name)["database"] == name
    assert urlsplit(environment(harness)["APP_POSTGRES_URI"]).path == f"/{name}"
    mark_ready(harness)
    assert isinstance(harness.model.unit.status, ops.ActiveStatus)


@pytest.mark.parametrize(
    "harness",
    [
        {"database-name": name}
        for name in ("", " leading", "-leading", "a" * 64, "db/name", "db?x=1", "字")
    ],
    indirect=True,
)
def test_invalid_initial_database_name_blocks_without_request(harness):
    relation_id = database(harness)
    assert isinstance(harness.model.unit.status, ops.BlockedStatus)
    assert "database-name" in harness.model.unit.status.message
    assert "database" not in harness.get_relation_data(relation_id, harness.model.app.name)
    assert_stopped(harness)


@pytest.mark.parametrize("leader", [True, False])
def test_database_name_change_on_existing_relation_blocks_without_retargeting(harness, leader):
    relation_id = database(harness)
    jwt = environment(harness)["APP_JWT_SECRET"]
    harness.set_leader(leader)
    harness.update_config({"database-name": "stg-app-center-rating"})
    assert isinstance(harness.model.unit.status, ops.BlockedStatus)
    assert "cannot change" in harness.model.unit.status.message
    assert harness.get_relation_data(relation_id, harness.model.app.name)["database"] == DATABASE
    assert_stopped(harness)
    harness.update_config({"database-name": DATABASE})
    assert urlsplit(environment(harness)["APP_POSTGRES_URI"]).path == "/ratings"
    assert environment(harness)["APP_JWT_SECRET"] == jwt


@pytest.mark.parametrize("name", ["stg-app-center-rating", "", "bad/name"])
def test_fresh_leader_relation_created_preserves_existing_request(harness, name):
    fresh = Harness(
        RatingsCharm,
        meta=yaml.safe_dump(METADATA),
        config=yaml.safe_dump(METADATA["config"]),
        actions=yaml.safe_dump(METADATA["actions"]),
    )
    try:
        fresh.set_leader(True)
        fresh.update_config({"database-name": name})
        peer_id = fresh.add_relation(PEERS, fresh.model.app.name)
        relation_id = fresh.add_relation("database", "postgresql-k8s")
        fresh.update_relation_data(peer_id, fresh.model.app.name, {"database-name": DATABASE})
        fresh.update_relation_data(relation_id, fresh.model.app.name, {"database": DATABASE})
        fresh.begin()
        fresh.set_can_connect(SERVICE, True)
        fresh.charm.container.add_layer(
            "image",
            {"services": {SERVICE: {"override": "replace", "command": "/bin/ratings"}}},
        )
        fresh.charm.on.database_relation_created.emit(
            fresh.model.get_relation("database", relation_id),
            fresh.model.get_app("postgresql-k8s"),
        )
        assert fresh.get_relation_data(relation_id, fresh.model.app.name)["database"] == DATABASE
        assert fresh.get_relation_data(peer_id, fresh.model.app.name)["database-name"] == DATABASE
        assert isinstance(fresh.model.unit.status, ops.BlockedStatus)
        assert_stopped(fresh)
    finally:
        fresh.cleanup()


def test_configured_database_rejects_mismatched_provider(harness):
    harness.update_config({"database-name": "stg-app-center-rating"})
    database(harness, DB_DATA | {"database": "prod-app-center-rating"})
    assert isinstance(harness.model.unit.status, ops.BlockedStatus)
    assert "other than database-name" in harness.model.unit.status.message
    assert_stopped(harness)


def test_configured_database_without_provider_database_field_uses_requested_name(harness):
    harness.update_config({"database-name": "stg-app-center-rating"})
    data = dict(DB_DATA)
    del data["database"]
    database(harness, data)
    assert urlsplit(environment(harness)["APP_POSTGRES_URI"]).path == "/stg-app-center-rating"


def test_becoming_leader_initializes_configured_database_request(harness):
    harness.set_leader(False)
    harness.update_config({"database-name": "stg-app-center-rating"})
    relation_id = harness.add_relation("database", "postgresql-k8s")
    harness.set_leader(True)
    assert harness.get_relation_data(relation_id, harness.model.app.name)["database"] == (
        "stg-app-center-rating"
    )


def test_invalid_database_config_change_does_not_publish_a_default_request(harness):
    harness.update_config({"database-name": "bad/name"})
    relation_id = database(harness)
    assert "database" not in harness.get_relation_data(relation_id, harness.model.app.name)
    assert isinstance(harness.model.unit.status, ops.BlockedStatus)
    assert_stopped(harness)


def test_no_database_is_blocked_and_image_autostart_is_disabled(harness):
    assert isinstance(harness.model.unit.status, ops.BlockedStatus)
    assert_stopped(harness)
    assert harness.model.get_secret(label=JWT_LABEL).peek_content()["jwt-secret"]


def test_real_image_service_is_stopped_without_credentials(harness):
    c = harness.charm.container
    c.add_layer(
        "image",
        {
            "services": {
                SERVICE: {"override": "replace", "startup": "enabled", "command": "/bin/ratings"}
            }
        },
        combine=True,
    )
    c.start(SERVICE)
    harness.charm.on.ratings_pebble_ready.emit(c)
    assert_stopped(harness)


def test_incomplete_database_waits_and_does_not_start(harness):
    database(harness, {"endpoints": DB_DATA["endpoints"]})
    assert isinstance(harness.model.unit.status, ops.WaitingStatus)
    assert "database credentials" in harness.model.unit.status.message
    assert_stopped(harness)


def test_configuration_and_port(harness):
    database(harness)
    env = environment(harness)
    assert env["APP_HOST"] == "0.0.0.0"
    assert env["APP_PORT"] == "8080"
    assert env["APP_SNAPCRAFT_IO_URI"] == "https://api.snapcraft.io/v2/"
    assert env["RUST_LOG"] == "info,sqlx=warn,hyper=warn,reqwest=warn"
    assert not any(key.startswith("APP_TLS") for key in env)
    assert (
        env["APP_JWT_SECRET"]
        == harness.model.get_secret(label=JWT_LABEL).peek_content()["jwt-secret"]
    )
    assert harness.charm.container.get_service(SERVICE).is_running()
    assert harness.model.unit.opened_ports() == {ops.Port("tcp", PORT)}
    uri = urlsplit(env["APP_POSTGRES_URI"])
    assert unquote(uri.username) == DB_DATA["username"]
    assert unquote(uri.password) == DB_DATA["password"]
    assert uri.path == "/ratings"
    assert parse_qs(uri.query) == {"sslmode": ["disable"]}
    plan = harness.get_container_pebble_plan(SERVICE)
    assert plan.checks[READY_CHECK].tcp == {"host": "127.0.0.1", "port": 8080}
    assert plan.checks[READY_CHECK].level == pebble.CheckLevel.READY


def test_active_requires_running_service_and_successful_check(harness):
    database(harness)
    with patch("charm.socket.create_connection", side_effect=ConnectionRefusedError):
        harness.charm.on.update_status.emit()
    assert isinstance(harness.model.unit.status, ops.WaitingStatus)
    mark_ready(harness)
    assert isinstance(harness.model.unit.status, ops.ActiveStatus)


@pytest.mark.parametrize(
    "status,successes",
    [(pebble.CheckStatus.DOWN, 0), (pebble.CheckStatus.DOWN, 21), (pebble.CheckStatus.INACTIVE, 1)],
)
def test_unready_checks_never_active(harness, status, successes):
    database(harness)
    info = pebble.CheckInfo(READY_CHECK, pebble.CheckLevel.READY, status, successes=successes)
    with (
        patch.object(ops.Container, "get_checks", return_value={READY_CHECK: info}),
        patch("charm.socket.create_connection", return_value=MagicMock()) as connect,
    ):
        harness.charm.on.update_status.emit()
    connect.assert_not_called()
    assert isinstance(harness.model.unit.status, ops.WaitingStatus)


def test_missing_check_never_active(harness):
    database(harness)
    with patch.object(ops.Container, "get_checks", return_value={}):
        harness.charm.on.update_status.emit()
    assert isinstance(harness.model.unit.status, ops.WaitingStatus)


def test_stopped_service_never_active_even_with_healthy_check(harness):
    database(harness)
    with patch.object(ops.Container, "replan"):
        harness.charm.container.stop(SERVICE)
        mark_ready(harness)
    assert isinstance(harness.model.unit.status, ops.WaitingStatus)
    assert "service to run" in harness.model.unit.status.message


@pytest.mark.parametrize("successes", [0, None])
def test_initial_up_check_with_confirmed_tcp_is_active(harness, successes):
    info = pebble.CheckInfo(
        READY_CHECK, pebble.CheckLevel.READY, pebble.CheckStatus.UP, successes=successes
    )
    with (
        patch.object(ops.Container, "get_checks", return_value={READY_CHECK: info}),
        patch("charm.socket.create_connection", return_value=MagicMock()) as connect,
    ):
        database(harness)
    connect.assert_called_once_with(("127.0.0.1", PORT), timeout=2)
    assert isinstance(harness.model.unit.status, ops.ActiveStatus)


@pytest.mark.parametrize("successes", [0, None])
@pytest.mark.parametrize("error", [ConnectionRefusedError, TimeoutError])
def test_initial_up_check_without_tcp_is_waiting(harness, successes, error):
    info = pebble.CheckInfo(
        READY_CHECK, pebble.CheckLevel.READY, pebble.CheckStatus.UP, successes=successes
    )
    with (
        patch.object(ops.Container, "get_checks", return_value={READY_CHECK: info}),
        patch("charm.socket.create_connection", side_effect=error) as connect,
    ):
        database(harness)
    connect.assert_called_once_with(("127.0.0.1", PORT), timeout=2)
    assert isinstance(harness.model.unit.status, ops.WaitingStatus)


def test_successful_check_needs_no_additional_tcp_probe(harness):
    database(harness)
    with patch("charm.socket.create_connection") as connect:
        mark_ready(harness)
    connect.assert_not_called()
    assert isinstance(harness.model.unit.status, ops.ActiveStatus)


def test_check_failure_and_recovery_events(harness):
    database(harness)
    for event_name, status in [
        ("ratings_pebble_check_failed", pebble.CheckStatus.DOWN),
        ("ratings_pebble_check_recovered", pebble.CheckStatus.UP),
    ]:
        info = pebble.CheckInfo(READY_CHECK, pebble.CheckLevel.READY, status, successes=1)
        with (
            patch.object(ops.Container, "get_checks", return_value={READY_CHECK: info}),
            patch.object(ops.Container, "stop") as stop,
            patch.object(ops.Container, "restart") as restart,
            patch.object(ops.Container, "stop_checks") as stop_checks,
        ):
            getattr(harness.charm.on, event_name).emit(harness.charm.container, READY_CHECK)
        stop.assert_not_called()
        restart.assert_not_called()
        stop_checks.assert_not_called()
        assert isinstance(
            harness.model.unit.status,
            ops.ActiveStatus if status == pebble.CheckStatus.UP else ops.WaitingStatus,
        )


def test_recovering_stopped_service_resets_its_old_readiness(harness):
    database(harness)
    harness.charm.container.stop(SERVICE)
    info = pebble.CheckInfo(
        READY_CHECK, pebble.CheckLevel.READY, pebble.CheckStatus.UP, successes=21
    )

    def reset_check(_container, *names):
        assert names == (READY_CHECK,)
        info.successes = 0

    with (
        patch.object(ops.Container, "get_checks", return_value={READY_CHECK: info}),
        patch.object(ops.Container, "stop_checks", autospec=True, side_effect=reset_check) as reset,
        patch("charm.socket.create_connection", side_effect=ConnectionRefusedError) as connect,
    ):
        harness.charm.on.update_status.emit()
    reset.assert_called_once()
    connect.assert_called_once_with(("127.0.0.1", PORT), timeout=2)
    assert harness.charm.container.get_service(SERVICE).is_running()
    assert isinstance(harness.model.unit.status, ops.WaitingStatus)


def test_unusable_jwt_content_blocks_and_stops(harness):
    database(harness)
    secret_id = harness.model.get_relation(PEERS).data[harness.model.app][JWT_ID]
    harness.model.get_secret(id=secret_id).set_content({"wrong-key": "value"})
    harness.charm.on.update_status.emit()
    assert isinstance(harness.model.unit.status, ops.BlockedStatus)
    assert_stopped(harness)


def test_jwt_length_and_no_plaintext_peer_copy(harness):
    secret = harness.model.get_secret(label=JWT_LABEL)
    jwt = secret.peek_content()["jwt-secret"]
    assert len(jwt) == 64  # 48 random bytes, standard Base64 without padding.
    assert len(base64.b64decode(jwt, validate=True)) == 48
    peer = harness.model.get_relation(PEERS)
    assert jwt not in json.dumps(dict(peer.data[harness.model.app]))


def test_jwt_creation_and_rotation_use_standard_not_urlsafe_base64(request):
    creation_bytes = b"\xfb\xff\xff" * 16
    rotation_bytes = b"\xff\xfb\xff" * 16
    with (
        patch("charm.secrets.token_bytes", side_effect=[creation_bytes, rotation_bytes]) as entropy,
        patch("charm.secrets.token_hex", return_value="generation"),
        patch("charm.secrets.token_urlsafe", side_effect=AssertionError("URL-safe JWT key")),
    ):
        # Initialise the Harness under the patch so creation is deterministic too.
        h = request.getfixturevalue("harness")
        created = h.model.get_secret(label=JWT_LABEL).peek_content()["jwt-secret"]
        h.run_action("rotate-jwt-secret")
        rotated = h.model.get_secret(label=JWT_LABEL).peek_content()["jwt-secret"]
    assert entropy.call_args_list == [call(48), call(48)]
    for value, raw in ((created, creation_bytes), (rotated, rotation_bytes)):
        assert len(base64.b64decode(value, validate=True)) == 48
        assert base64.b64decode(value, validate=True) == raw
        assert value == base64.b64encode(raw).decode("ascii")
        assert "+" in value or "/" in value
        assert "-" not in value and "_" not in value
        assert value != base64.urlsafe_b64encode(raw).decode("ascii")


def test_file_permission_errors_are_not_hidden(harness):
    database(harness)
    with patch.object(
        ops.Container, "pull", side_effect=pebble.PathError("permission-denied", "private")
    ):
        with pytest.raises(pebble.PathError):
            harness.update_relation_data(
                harness.model.get_relation("database").id,
                "postgresql-k8s",
                {"tls": "True", "tls-ca": "CA"},
            )


def test_db_secret_refresh_permission_failure_stops_workload(harness):
    relation_id = database(harness, secret=True)
    secret_id = harness.get_relation_data(relation_id, "postgresql-k8s")["secret-user"]
    with patch.object(ops.Secret, "get_content", side_effect=ops.ModelError("permission denied")):
        harness.charm.on.secret_changed.emit(secret_id, None)
    assert isinstance(harness.model.unit.status, ops.WaitingStatus)
    assert_stopped(harness)


def test_unchanged_config_does_not_restart(harness):
    database(harness)
    with patch.object(ops.Container, "stop") as stop:
        harness.charm.on.config_changed.emit()
        harness.charm.on.update_status.emit()
    stop.assert_not_called()


def test_config_change_restarts_and_retains_jwt(harness):
    database(harness)
    jwt = environment(harness)["APP_JWT_SECRET"]
    with patch.object(ops.Container, "stop", autospec=True) as stop:
        harness.update_config({"snap-store-url": "http://store.local:8000/v2/", "rust-log": "warn"})
    stop.assert_called_once()
    assert environment(harness)["APP_JWT_SECRET"] == jwt
    assert environment(harness)["RUST_LOG"] == "warn"
    assert environment(harness)["APP_SNAPCRAFT_IO_URI"] == "http://store.local:8000/v2/"


@pytest.mark.parametrize(
    "url",
    [
        "",
        "api.snapcraft.io/v2/",
        "ftp://store/v2/",
        "https://user:password@store/v2/",
        "https://store/v2",
        "https://store/v2/?q=1",
        "https://store/v2/#x",
        "https://store:bad/v2/",
        "https://[bad/v2/",
        "https://store/v 2/",
    ],
)
def test_invalid_store_url_blocks_and_stops(harness, url):
    database(harness)
    harness.update_config({"snap-store-url": url})
    assert isinstance(harness.model.unit.status, ops.BlockedStatus)
    assert_stopped(harness)
    harness.update_config({"snap-store-url": "https://api.snapcraft.io/v2/"})
    assert harness.charm.container.get_service(SERVICE).is_running()


@pytest.mark.parametrize("value", ["", "   ", "info\ntrace", "info\rtrace"])
def test_invalid_log_filter_blocks(harness, value):
    database(harness)
    harness.update_config({"rust-log": value})
    assert isinstance(harness.model.unit.status, ops.BlockedStatus)
    assert_stopped(harness)


def test_database_relation_removal_drops_stale_config_and_can_recover(harness):
    relation_id = database(harness)
    mark_ready(harness)
    with patch(
        "charm.DatabaseRequirerData.fetch_relation_data",
        side_effect=AssertionError("must not fetch on broken"),
    ):
        harness.remove_relation(relation_id)
    assert isinstance(harness.model.unit.status, ops.BlockedStatus)
    assert_stopped(harness)
    database(harness)
    mark_ready(harness)
    assert isinstance(harness.model.unit.status, ops.ActiveStatus)


def test_database_credentials_disappear_stops_workload(harness):
    relation_id = database(harness)
    harness.update_relation_data(relation_id, "postgresql-k8s", {"password": ""})
    assert isinstance(harness.model.unit.status, ops.WaitingStatus)
    assert_stopped(harness)


def test_endpoints_and_plain_credentials_changes_restart(harness):
    relation_id = database(harness)
    with patch.object(ops.Container, "stop", autospec=True) as stop:
        harness.update_relation_data(
            relation_id, "postgresql-k8s", {"endpoints": "new-primary:5433"}
        )
    stop.assert_called_once()
    assert "@new-primary:5433/ratings" in environment(harness)["APP_POSTGRES_URI"]
    harness.update_relation_data(relation_id, "postgresql-k8s", {"password": "newpass"})
    assert ":newpass@" in environment(harness)["APP_POSTGRES_URI"]


def test_database_secrets_and_rotation_use_official_accessor(harness):
    relation_id = database(harness, secret=True)
    secret_id = harness.get_relation_data(relation_id, "postgresql-k8s")["secret-user"]
    assert (
        unquote(urlsplit(environment(harness)["APP_POSTGRES_URI"]).password) == DB_DATA["password"]
    )
    harness.set_secret_content(secret_id, {"username": "relation-user", "password": "rotated:@!"})
    assert unquote(urlsplit(environment(harness)["APP_POSTGRES_URI"]).password) == "rotated:@!"


def test_tls_ca_secret_is_consumed_and_rotated(harness):
    relation_id = database(harness)
    secret_id = harness.add_model_secret("postgresql-k8s", {"tls": "True", "tls-ca": "CA-one"})
    harness.grant_secret(secret_id, harness.model.app)
    harness.update_relation_data(
        relation_id, "postgresql-k8s", {"tls": "", "secret-tls": secret_id}
    )
    assert harness.charm.container.pull(CA_PATH).read() == "CA-one"
    query = parse_qs(urlsplit(environment(harness)["APP_POSTGRES_URI"]).query)
    assert query == {"sslmode": ["verify-full"], "sslrootcert": [CA_PATH]}
    with patch.object(ops.Container, "stop", autospec=True) as stop:
        harness.set_secret_content(secret_id, {"tls": "True", "tls-ca": "CA-two"})
    stop.assert_called_once()
    assert harness.charm.container.pull(CA_PATH).read() == "CA-two"
    harness.set_secret_content(secret_id, {"tls": "False", "tls-ca": "CA-two"})
    assert not harness.charm.container.exists(CA_PATH)
    assert "sslmode=disable" in environment(harness)["APP_POSTGRES_URI"]


def test_tls_enabled_without_ca_blocks_instead_of_downgrading(harness):
    database(harness, DB_DATA | {"tls": "True"})
    assert isinstance(harness.model.unit.status, ops.BlockedStatus)
    assert "tls-ca" in harness.model.unit.status.message
    assert_stopped(harness)


def test_jwt_is_stable_across_hooks_and_leader_failover(harness):
    database(harness)
    old = environment(harness)["APP_JWT_SECRET"]
    peer = harness.model.get_relation(PEERS)
    secret_id = peer.data[harness.model.app][JWT_ID]
    for event in (
        harness.charm.on.upgrade_charm,
        harness.charm.on.start,
        harness.charm.on.update_status,
    ):
        event.emit()
    harness.set_leader(False)
    harness.charm.on.config_changed.emit()
    harness.set_leader(True)
    assert peer.data[harness.model.app][JWT_ID] == secret_id
    assert environment(harness)["APP_JWT_SECRET"] == old
    assert harness.get_secret_revisions(secret_id) == [1]


def test_leader_reuses_labelled_secret_if_publication_was_interrupted(harness):
    peer = harness.model.get_relation(PEERS)
    secret_id = peer.data[harness.model.app][JWT_ID]
    jwt = harness.model.get_secret(id=secret_id).peek_content()["jwt-secret"]
    peer.data[harness.model.app].pop(JWT_ID)
    harness.charm.on.leader_elected.emit()
    assert peer.data[harness.model.app][JWT_ID] == secret_id
    assert harness.model.get_secret(id=secret_id).peek_content()["jwt-secret"] == jwt


def test_follower_waits_then_uses_same_published_secret(harness):
    database(harness)
    peer = harness.model.get_relation(PEERS)
    secret_id = peer.data[harness.model.app][JWT_ID]
    jwt = environment(harness)["APP_JWT_SECRET"]
    peer.data[harness.model.app].pop(JWT_ID)
    harness.set_leader(False)
    harness.charm.on.start.emit()
    assert isinstance(harness.model.unit.status, ops.WaitingStatus)
    assert_stopped(harness)
    harness.update_relation_data(peer.id, harness.model.app.name, {JWT_ID: secret_id})
    assert environment(harness)["APP_JWT_SECRET"] == jwt
    assert harness.get_secret_revisions(secret_id) == [1]


def test_rotation_action_restarts_leader_and_peer_change_restarts_follower(harness):
    database(harness)
    peer = harness.model.get_relation(PEERS)
    secret_id = peer.data[harness.model.app][JWT_ID]
    old = environment(harness)["APP_JWT_SECRET"]
    result = harness.run_action("rotate-jwt-secret")
    assert result.results == {"rotated": True}
    new = environment(harness)["APP_JWT_SECRET"]
    assert new != old
    assert len(base64.b64decode(new, validate=True)) == 48
    assert peer.data[harness.model.app][JWT_ID] == secret_id
    assert harness.get_secret_revisions(secret_id) == [1, 2]
    assert peer.data[harness.model.app][JWT_GENERATION] != "0"
    # All units are owners, so simulate another leader's rotation with set_content,
    # then a peer-changed event (Juju emits no secret-changed for secret owners).
    harness.model.get_secret(id=secret_id).set_content({"jwt-secret": "another-leader-value"})
    harness.set_leader(False)
    harness.update_relation_data(peer.id, harness.model.app.name, {JWT_GENERATION: "next"})
    assert environment(harness)["APP_JWT_SECRET"] == "another-leader-value"


def test_rotation_fails_if_peer_disappears_during_action(harness):
    with patch.object(harness.charm, "_jwt_secret", return_value=None):
        with pytest.raises(ActionFailed, match="peer"):
            harness.run_action("rotate-jwt-secret")


def test_rotation_action_on_follower_fails_without_changes(harness):
    database(harness)
    old = environment(harness)["APP_JWT_SECRET"]
    harness.set_leader(False)
    with pytest.raises(ActionFailed, match="leader"):
        harness.run_action("rotate-jwt-secret")
    assert environment(harness)["APP_JWT_SECRET"] == old


def test_missing_published_jwt_waits_without_generating_a_replacement(harness):
    database(harness)
    peer = harness.model.get_relation(PEERS)
    secret_id = peer.data[harness.model.app][JWT_ID]
    harness.model.get_secret(id=secret_id).remove_all_revisions()
    harness.charm.on.update_status.emit()
    assert isinstance(harness.model.unit.status, ops.WaitingStatus)
    assert peer.data[harness.model.app][JWT_ID] == secret_id
    assert_stopped(harness)
    with pytest.raises(ActionFailed, match="unavailable"):
        harness.run_action("rotate-jwt-secret")


def test_jwt_secret_changed_reloads_latest(harness):
    database(harness)
    secret_id = harness.model.get_relation(PEERS).data[harness.model.app][JWT_ID]
    harness.model.get_secret(id=secret_id).set_content({"jwt-secret": "changed"})
    harness.charm.on.secret_changed.emit(secret_id, JWT_LABEL)
    assert environment(harness)["APP_JWT_SECRET"] == "changed"


def test_unrelated_secret_events_do_nothing(harness):
    database(harness)
    secret_id = harness.add_model_secret("unrelated", {"password": "value"})
    with patch.object(harness.charm, "_reconcile") as reconcile:
        harness.charm.on.secret_changed.emit(secret_id, None)
    reconcile.assert_not_called()


def test_no_peer_waits_and_rotation_fails(harness):
    peer = harness.model.get_relation(PEERS)
    harness.remove_relation(peer.id)
    assert_stopped(harness)
    harness.charm.on.update_status.emit()
    assert isinstance(harness.model.unit.status, ops.WaitingStatus)
    with pytest.raises(ActionFailed, match="peer"):
        harness.run_action("rotate-jwt-secret")


def test_becoming_leader_initializes_existing_database_request(harness):
    harness.set_leader(False)
    relation_id = harness.add_relation("database", "postgresql-k8s")
    assert "database" not in harness.get_relation_data(relation_id, harness.model.app.name)
    harness.set_leader(True)
    data = harness.get_relation_data(relation_id, harness.model.app.name)
    assert data["database"] == "ratings"
    assert "tls-ca" in json.loads(data["requested-secrets"])


def test_disconnected_pebble_does_not_open_port(harness):
    harness.set_can_connect(SERVICE, False)
    database(harness)
    assert isinstance(harness.model.unit.status, ops.WaitingStatus)
    assert not harness.model.unit.opened_ports()
    harness.container_pebble_ready(SERVICE)
    assert harness.charm.container.get_service(SERVICE).is_running()


def test_database_broken_while_pebble_disconnected_stops_when_available(harness):
    relation_id = database(harness)
    harness.set_can_connect(SERVICE, False)
    harness.remove_relation(relation_id)
    assert not harness.model.unit.opened_ports()
    assert isinstance(harness.model.unit.status, ops.WaitingStatus)
    harness.container_pebble_ready(SERVICE)
    assert_stopped(harness)
    assert isinstance(harness.model.unit.status, ops.BlockedStatus)


def test_stop_hook_disables_service_and_closes_port(harness):
    database(harness)
    harness.charm.on.stop.emit()
    assert isinstance(harness.model.unit.status, ops.MaintenanceStatus)
    assert_stopped(harness)


@pytest.mark.parametrize(
    "error", [pebble.ConnectionError("private"), pebble.TimeoutError("private")]
)
def test_expected_pebble_failures_are_safe_and_retried(harness, error, caplog):
    database(harness)
    with patch.object(ops.Container, "replan", side_effect=error):
        harness.charm.on.update_status.emit()
    assert not isinstance(harness.model.unit.status, ops.ActiveStatus)
    assert not harness.model.unit.opened_ports()
    assert "private" not in caplog.text
    mark_ready(harness)
    assert isinstance(harness.model.unit.status, ops.ActiveStatus)


def test_change_failure_does_not_log_workload_credentials(harness, caplog):
    database(harness)
    change = MagicMock(tasks=[])
    error = pebble.ChangeError("password=very-private", change)
    with patch.object(ops.Container, "replan", side_effect=error):
        harness.charm.on.update_status.emit()
    assert isinstance(harness.model.unit.status, ops.BlockedStatus)
    assert "very-private" not in caplog.text + harness.model.unit.status.message
    mark_ready(harness)
    assert isinstance(harness.model.unit.status, ops.ActiveStatus)


def test_invalid_config_and_failed_stop_does_not_swallow_pebble_error(harness):
    database(harness)
    with patch.object(ops.Container, "stop", side_effect=pebble.ConnectionError("private")):
        harness.update_config({"rust-log": ""})
    assert isinstance(harness.model.unit.status, ops.WaitingStatus)
    assert not harness.model.unit.opened_ports()


def test_programming_errors_are_not_hidden(harness):
    database(harness)
    with patch.object(ops.Container, "add_layer", side_effect=RuntimeError("bug")):
        with pytest.raises(RuntimeError, match="bug"):
            harness.charm.on.update_status.emit()


def test_unknown_model_errors_are_not_hidden(harness):
    with patch("charm.DatabaseRequirerData.fetch_relation_data", side_effect=ops.ModelError("bug")):
        with pytest.raises(ops.ModelError, match="bug"):
            database(harness)


def test_pending_secret_permissions_stops_workload(harness):
    database(harness)
    with patch(
        "charm.DatabaseRequirerData.fetch_relation_data",
        side_effect=ops.ModelError("permission denied"),
    ):
        harness.charm.on.update_status.emit()
    assert isinstance(harness.model.unit.status, ops.WaitingStatus)
    assert_stopped(harness)


def test_loki_forwarder_creates_labels_and_disables_removed_target(harness):
    database(harness)
    relation_id = harness.add_relation("logging", "loki")
    harness.add_relation_unit(relation_id, "loki/0")
    endpoint = "http://loki.cos-lite.svc.cluster.local:3100/loki/api/v1/push"
    harness.update_relation_data(relation_id, "loki/0", {"endpoint": json.dumps({"url": endpoint})})
    target = harness.get_container_pebble_plan(SERVICE).log_targets["loki/0"]
    assert target.location == endpoint
    assert target.services == ["all"]
    assert target.labels["juju_application"] == "ratings"
    assert target.labels["juju_unit"] == "ratings/0"
    assert "juju_model_uuid" in target.labels
    harness.remove_relation(relation_id)
    assert harness.get_container_pebble_plan(SERVICE).log_targets["loki/0"].services == ["-all"]
    assert harness.charm.container.get_service(SERVICE).is_running()


def test_loki_relation_before_container_ready_is_reconciled_on_pebble_ready(harness):
    harness.set_can_connect(SERVICE, False)
    relation_id = harness.add_relation("logging", "loki")
    harness.add_relation_unit(relation_id, "loki/0")
    harness.update_relation_data(
        relation_id,
        "loki/0",
        {"endpoint": json.dumps({"url": "http://loki:3100/loki/api/v1/push"})},
    )
    database(harness)
    harness.container_pebble_ready(SERVICE)
    assert "loki/0" in harness.get_container_pebble_plan(SERVICE).log_targets


def ingress_config(h, relation_id):
    return yaml.safe_load(h.get_relation_data(relation_id, h.model.app.name)["config"])


def expected_ingress(h, hostname):
    name = f"ratings-{h.model.uuid}-{h.model.app.name}-grpc"
    return {
        "http": {
            "routers": {
                name: {
                    "rule": f"Host(`{hostname}`)",
                    "entryPoints": ["websecure"],
                    "tls": {},
                    "service": name,
                }
            },
            "services": {
                name: {"loadBalancer": {"servers": [{"url": f"h2c://{h.model.app.name}:8080"}]}}
            },
        }
    }


def test_ingress_metadata_is_optional_and_defaults_to_no_hostname():
    assert METADATA["requires"][INGRESS] == {
        "interface": "traefik_route",
        "limit": 1,
        "optional": True,
    }
    assert METADATA["config"]["options"]["external-hostname"]["type"] == "string"
    assert METADATA["config"]["options"]["external-hostname"]["default"] == ""


@pytest.mark.parametrize("hostname", ["", "ratings.localhost", "not/a/hostname"])
def test_ingress_is_optional_and_unused_config_does_not_affect_backend(harness, hostname):
    database(harness)
    original_env = environment(harness)
    with patch.object(ops.Container, "stop") as stop:
        harness.update_config({"external-hostname": hostname})
        mark_ready(harness)
    stop.assert_not_called()
    assert isinstance(harness.model.unit.status, ops.ActiveStatus)
    assert environment(harness) == original_env
    assert harness.charm.traefik_route is None


@pytest.mark.parametrize("hostname", ["ratings.localhost", "ratings.staging.ubuntu.com"])
def test_ingress_publishes_exact_raw_tls_h2c_route_after_begin(harness, hostname, caplog):
    database(harness, secret=True)
    original_env = environment(harness)
    harness.update_config({"external-hostname": hostname})
    assert harness.charm.traefik_route is None
    relation_id = harness.add_relation(INGRESS, "traefik")
    assert isinstance(harness.charm.traefik_route, TraefikRouteRequirer)
    assert harness.charm.traefik_route.is_ready()
    assert ingress_config(harness, relation_id) == expected_ingress(harness, hostname)
    data = harness.get_relation_data(relation_id, harness.model.app.name)
    assert set(data) == {"raw", "config"}
    assert data["raw"] == "True"
    # Exact equality also excludes PathPrefix/StripPrefix, middleware, redirects,
    # HTTP fallback routers, static config and per-pod endpoints.
    exposed = json.dumps(data) + caplog.text
    for secret in (original_env["APP_JWT_SECRET"], DB_DATA["password"], DB_DATA["username"]):
        assert secret not in exposed
    assert "secret:" not in exposed
    assert environment(harness) == original_env
    mark_ready(harness)
    assert isinstance(harness.model.unit.status, ops.ActiveStatus)


@pytest.mark.parametrize(
    "app,model_uuid",
    [
        ("staging-ratings", "11111111-1111-4111-8111-111111111111"),
        ("other-ratings", "11111111-1111-4111-8111-111111111111"),
        ("staging-ratings", "22222222-2222-4222-8222-222222222222"),
    ],
)
def test_ingress_uses_model_app_identity_and_actual_service_name(harness, app, model_uuid):
    fresh = Harness(
        RatingsCharm,
        meta=yaml.safe_dump(METADATA | {"name": app}),
        config=yaml.safe_dump(METADATA["config"]),
    )
    try:
        fresh.set_model_uuid(model_uuid)
        fresh.set_leader(True)
        fresh.update_config({"external-hostname": "ratings.staging.ubuntu.com"})
        # Also cover a relation that already exists when the charm initialises.
        relation_id = fresh.add_relation(INGRESS, "shared-traefik")
        fresh.begin()
        fresh.charm.on.upgrade_charm.emit()
        config = ingress_config(fresh, relation_id)
        assert config == expected_ingress(fresh, "ratings.staging.ubuntu.com")
        assert list(config["http"]["routers"]) == [f"ratings-{model_uuid}-{app}-grpc"]
    finally:
        fresh.cleanup()


@pytest.mark.parametrize(
    "hostname",
    [
        "ratings.localhost",
        "RATINGS.Staging.Ubuntu.COM",
        "a",
        "0-1.example",
        "xn--bcher-kva.example",
        "a" * 63 + ".example",
        ".".join(["a" * 63] * 3 + ["b" * 61]),
    ],
)
def test_ingress_accepts_dns_boundaries_and_normalises_case(harness, hostname):
    assert validate_external_hostname(hostname) == hostname.lower()
    harness.update_config({"external-hostname": hostname})
    relation_id = harness.add_relation(INGRESS, "traefik")
    assert ingress_config(harness, relation_id) == expected_ingress(harness, hostname.lower())


@pytest.mark.parametrize(
    "hostname",
    [
        "",
        "https://ratings.localhost",
        "ratings.localhost:443",
        "ratings.localhost/path",
        "*.ratings.localhost",
        "ratings.localhost`) || Host(`attacker.example",
        "ratings.localhost`",
        "ratings.localhost?foo=bar",
        "ratings.localhost#fragment",
        "user@ratings.localhost",
        " ratings.localhost",
        "ratings.localhost ",
        "ratings.localhost\n",
        "ratings.\nlocalhost",
        ".ratings.localhost",
        "ratings.localhost.",
        "ratings..localhost",
        "-ratings.localhost",
        "ratings-.localhost",
        "ratings_localhost",
        "ratíngs.localhost",
        "K.localhost",
        "ratings\x00.localhost",
        "a" * 64 + ".localhost",
        ".".join(["a" * 63] * 3 + ["b" * 62]),
    ],
)
def test_ingress_bad_hostname_withdraws_stale_route_and_recovers(harness, hostname, caplog):
    database(harness)
    harness.update_config({"external-hostname": "ratings.localhost"})
    relation_id = harness.add_relation(INGRESS, "traefik")
    original_env = environment(harness)
    with patch.object(ops.Container, "stop") as stop:
        harness.update_config({"external-hostname": hostname})
        mark_ready(harness)
        assert isinstance(harness.model.unit.status, ops.BlockedStatus)
        assert "external-hostname" in harness.model.unit.status.message
        assert ingress_config(harness, relation_id) == {"http": {"routers": {}, "services": {}}}
        assert harness.get_relation_data(relation_id, harness.model.app.name)["raw"] == "True"
        assert harness.charm.container.get_service(SERVICE).is_running()
        assert environment(harness) == original_env
        harness.update_config({"external-hostname": "ratings.staging.ubuntu.com"})
        mark_ready(harness)
    stop.assert_not_called()
    assert isinstance(harness.model.unit.status, ops.ActiveStatus)
    assert ingress_config(harness, relation_id) == expected_ingress(
        harness, "ratings.staging.ubuntu.com"
    )
    assert original_env["APP_JWT_SECRET"] not in caplog.text
    assert DB_DATA["password"] not in caplog.text


@pytest.mark.parametrize("leader", [True, False])
def test_ingress_related_without_hostname_blocks_but_keeps_backend(harness, leader):
    database(harness)
    harness.set_leader(leader)
    relation_id = harness.add_relation(INGRESS, "traefik")
    mark_ready(harness)
    assert isinstance(harness.model.unit.status, ops.BlockedStatus)
    assert (
        harness.model.unit.status.message == "Set external-hostname when traefik-route is related"
    )
    assert harness.charm.container.get_service(SERVICE).is_running()
    if leader:
        assert ingress_config(harness, relation_id) == {"http": {"routers": {}, "services": {}}}
    else:
        assert harness.get_relation_data(relation_id, harness.model.app.name) == {}


def test_ingress_hostname_change_does_not_restart_or_rotate_workload(harness):
    database(harness)
    harness.update_config({"external-hostname": "ratings.localhost"})
    relation_id = harness.add_relation(INGRESS, "traefik")
    original_env = environment(harness)
    with (
        patch.object(ops.Container, "stop") as stop,
        patch.object(ops.Container, "restart") as restart,
    ):
        harness.update_config({"external-hostname": "ratings.staging.ubuntu.com"})
        mark_ready(harness)
    stop.assert_not_called()
    restart.assert_not_called()
    assert environment(harness) == original_env
    assert ingress_config(harness, relation_id) == expected_ingress(
        harness, "ratings.staging.ubuntu.com"
    )


def test_ingress_follower_does_not_publish_and_leader_elected_initialises(harness):
    database(harness)
    harness.set_leader(False)
    harness.update_config({"external-hostname": "ratings.localhost"})
    relation_id = harness.add_relation(INGRESS, "traefik")
    harness.add_relation_unit(relation_id, "traefik/0")
    harness.update_relation_data(relation_id, "traefik", {"external_host": "ignored.example"})
    harness.charm.on.upgrade_charm.emit()
    assert harness.get_relation_data(relation_id, harness.model.app.name) == {}
    assert harness.charm.traefik_route is None
    harness.set_leader(True)
    assert ingress_config(harness, relation_id) == expected_ingress(harness, "ratings.localhost")
    harness.set_leader(False)
    harness.update_config({"external-hostname": "ratings.staging.ubuntu.com"})
    assert ingress_config(harness, relation_id) == expected_ingress(harness, "ratings.localhost")
    harness.set_leader(True)
    assert ingress_config(harness, relation_id) == expected_ingress(
        harness, "ratings.staging.ubuntu.com"
    )


def test_ingress_invalid_config_on_follower_is_withdrawn_when_elected(harness):
    database(harness)
    harness.update_config({"external-hostname": "ratings.localhost"})
    relation_id = harness.add_relation(INGRESS, "traefik")
    original_data = harness.get_relation_data(relation_id, harness.model.app.name).copy()
    harness.set_leader(False)
    harness.update_config({"external-hostname": "invalid`hostname"})
    mark_ready(harness)
    assert isinstance(harness.model.unit.status, ops.BlockedStatus)
    assert harness.get_relation_data(relation_id, harness.model.app.name) == original_data
    harness.set_leader(True)
    assert ingress_config(harness, relation_id) == {"http": {"routers": {}, "services": {}}}
    harness.update_config({"external-hostname": "ratings.staging.ubuntu.com"})
    assert ingress_config(harness, relation_id) == expected_ingress(
        harness, "ratings.staging.ubuntu.com"
    )


@pytest.mark.parametrize("event", ["joined", "changed", "upgrade", "leader"])
def test_ingress_events_republish_from_current_config(harness, event):
    harness.update_config({"external-hostname": "ratings.localhost"})
    relation_id = harness.add_relation(INGRESS, "traefik")
    harness.update_relation_data(relation_id, harness.model.app.name, {"config": ""})
    if event == "joined":
        harness.add_relation_unit(relation_id, "traefik/0")
    elif event == "changed":
        harness.update_relation_data(relation_id, "traefik", {"external_host": "ignored.example"})
    elif event == "upgrade":
        harness.charm.on.upgrade_charm.emit()
    else:
        harness.set_leader(True)
    assert ingress_config(harness, relation_id) == expected_ingress(harness, "ratings.localhost")


@pytest.mark.parametrize("hostname", ["ratings.localhost", ""])
def test_ingress_broken_never_writes_dying_relation_or_stops_backend_and_can_recreate(
    harness, hostname
):
    database(harness)
    harness.update_config({"external-hostname": hostname})
    relation_id = harness.add_relation(INGRESS, "traefik")
    harness.add_relation_unit(relation_id, "traefik/0")
    old_requirer = harness.charm.traefik_route
    original_env = environment(harness)
    with (
        patch.object(
            old_requirer, "submit_to_traefik", side_effect=AssertionError("dying relation")
        ),
        patch.object(ops.Container, "stop") as stop,
    ):
        harness.remove_relation(relation_id)
        mark_ready(harness)
        assert isinstance(harness.model.unit.status, ops.ActiveStatus)
        harness.update_config({"external-hostname": "ratings.staging.ubuntu.com"})
        new_id = harness.add_relation(INGRESS, "replacement-traefik")
        harness.add_relation_unit(new_id, "replacement-traefik/0")
        harness.update_relation_data(new_id, "replacement-traefik", {"scheme": "http"})
        assert harness.charm.traefik_route is not old_requirer
        assert isinstance(harness.charm.traefik_route, TraefikRouteRequirer)
        assert ingress_config(harness, new_id) == expected_ingress(
            harness, "ratings.staging.ubuntu.com"
        )
    stop.assert_not_called()
    assert environment(harness) == original_env
    assert harness.model.unit.opened_ports() == {ops.Port("tcp", PORT)}


def test_ingress_does_not_withdraw_for_transient_backend_health(harness):
    database(harness)
    harness.update_config({"external-hostname": "ratings.localhost"})
    relation_id = harness.add_relation(INGRESS, "traefik")
    original_config = ingress_config(harness, relation_id)
    harness.charm.on.ratings_pebble_check_failed.emit(harness.charm.container, READY_CHECK)
    assert isinstance(harness.model.unit.status, ops.WaitingStatus)
    assert ingress_config(harness, relation_id) == original_config
    harness.set_can_connect(SERVICE, False)
    harness.charm.on.update_status.emit()
    assert ingress_config(harness, relation_id) == original_config
    # Hostname withdrawal must work even when Pebble cannot be reached.
    harness.update_config({"external-hostname": ""})
    assert isinstance(harness.model.unit.status, ops.BlockedStatus)
    assert ingress_config(harness, relation_id) == {"http": {"routers": {}, "services": {}}}


@pytest.mark.parametrize(
    "username,password",
    [("u:@/?#% +", "p:@/?#% +"), ("名字", "päss字"), ("%40", "%2F"), ("user", "pass")],
)
def test_uri_credentials_round_trip(username, password):
    uri, ca = postgres_uri(DB_DATA | {"username": username, "password": password})
    parsed = urlsplit(uri)
    assert unquote(parsed.username) == username
    assert unquote(parsed.password) == password
    assert ca is None


def test_uri_uses_configured_database_with_tls():
    name = "stg-app-center-rating"
    uri, ca = postgres_uri(DB_DATA | {"database": name, "tls": "true", "tls-ca": "test-ca"}, name)
    assert urlsplit(uri).path == f"/{name}"
    assert parse_qs(urlsplit(uri).query)["sslmode"] == ["verify-full"]
    assert ca == "test-ca"


@pytest.mark.parametrize("name", ["bad/name", "a" * 64])
def test_uri_rejects_invalid_requested_database_name(name):
    with pytest.raises(ConfigurationError, match="database-name"):
        postgres_uri(DB_DATA, name)


def test_uri_uses_first_primary_endpoint_and_bracketed_ipv6():
    uri, _ = postgres_uri(DB_DATA | {"endpoints": "[2001:db8::1]:5432,second:5432"})
    assert urlsplit(uri).hostname == "2001:db8::1"


@pytest.mark.parametrize(
    "endpoint",
    [
        "bad",
        "host:0",
        "host:65536",
        "host:abc",
        "postgresql://host:5432",
        "user@host:5432",
        "host:5432/path",
        "host:5432?sslmode=disable",
        "2001:db8::1:5432",
    ],
)
def test_invalid_endpoints_are_rejected(endpoint):
    with pytest.raises(ConfigurationError):
        postgres_uri(DB_DATA | {"endpoints": endpoint})


@pytest.mark.parametrize("updates", [{"tls": "maybe"}, {"database": "postgres"}])
def test_invalid_provider_data_is_rejected(updates):
    with pytest.raises(ConfigurationError):
        postgres_uri(DB_DATA | updates)


def test_multiple_database_relations_block(harness):
    database(harness)
    relation_id = harness.add_relation("database", "other-db")
    harness.update_relation_data(relation_id, "other-db", DB_DATA)
    assert isinstance(harness.model.unit.status, ops.BlockedStatus)
    assert_stopped(harness)


def test_library_lock_checksums_and_metadata():
    import ast
    import hashlib

    root = Path(__file__).parents[2]
    for entry in json.loads((root / "libraries.lock.json").read_text()):
        data = (root / entry["path"]).read_bytes()
        assert hashlib.sha256(data).hexdigest() == entry["sha256"]
        assignments = {
            node.targets[0].id: ast.literal_eval(node.value)
            for node in ast.parse(data).body
            if isinstance(node, ast.Assign)
            and isinstance(node.targets[0], ast.Name)
            and node.targets[0].id in {"LIBID", "LIBAPI", "LIBPATCH", "PYDEPS"}
        }
        assert all(entry[key] == value for key, value in assignments.items())
        assert entry["commit"] in entry["source"]
