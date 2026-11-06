#!/usr/bin/env python3
"""Local sidecar operator for Ratings; no database credentials are stored by the charm."""

import base64
import json
import re
import secrets
import socket
from typing import cast
from urllib.parse import quote, urlencode, urlsplit

import ops
from charms.data_platform_libs.v0.data_interfaces import DatabaseRequirerData, DatabaseRequires
from charms.loki_k8s.v1.loki_push_api import LogForwarder
from ops import pebble

SERVICE = "ratings"
PORT = 8080
DATABASE = "ratings"
PEERS = "ratings-peers"
JWT_LABEL = "ratings-jwt"
JWT_ID = "jwt-secret-id"
JWT_GENERATION = "jwt-generation"
READY_CHECK = "ratings-ready"
CA_PATH = "/etc/ratings/postgresql-ca.crt"


class ConfigurationError(ValueError):
    """Invalid operator configuration or provider connection data (safe to display)."""


def postgres_uri(data: dict[str, str]) -> tuple[str, str | None]:
    """Build a SQLx URI from the official relation fields, not provider admin credentials."""
    endpoint = data["endpoints"].split(",")[0].strip()
    # Endpoints are host:port, not arbitrary URLs. Restrict authority syntax so
    # malformed provider data cannot inject user-info, a query, or another host.
    if not re.fullmatch(r"(?:[a-zA-Z0-9_.-]+|\[[0-9a-fA-F:]+\]):[0-9]+", endpoint):
        raise ConfigurationError("Database endpoint must be host:port (IPv6 must be bracketed)")
    if not 1 <= int(endpoint.rsplit(":", 1)[1]) <= 65535:
        raise ConfigurationError("Database endpoint port is out of range")
    if data.get("database", DATABASE) != DATABASE:
        raise ConfigurationError("Database provider returned a database other than ratings")
    tls = data.get("tls", "false").lower()
    if tls not in {"true", "false"}:
        raise ConfigurationError("Database provider returned an invalid TLS flag")
    ca = data.get("tls-ca") if tls == "true" else None
    if tls == "true" and not ca:
        raise ConfigurationError("Database TLS is enabled but the provider has not supplied tls-ca")
    query = {"sslmode": "verify-full" if ca else "disable"}
    if ca:
        query["sslrootcert"] = CA_PATH
    username = quote(data["username"], safe="")
    password = quote(data["password"], safe="")
    return f"postgresql://{username}:{password}@{endpoint}/{DATABASE}?{urlencode(query)}", ca


class RatingsCharm(ops.CharmBase):
    def __init__(self, *args):
        super().__init__(*args)
        self.container = self.unit.get_container(SERVICE)
        self.database = DatabaseRequires(self, "database", database_name=DATABASE)
        self.log_forwarder = LogForwarder(self, relation_name="logging", forward_alert_rules=False)
        for event in (
            self.on.install,
            self.on.start,
            self.on.config_changed,
            self.on.upgrade_charm,
            self.on.update_status,
            self.on.leader_elected,
            self.on.ratings_pebble_ready,
            self.on[PEERS].relation_created,
            self.on[PEERS].relation_joined,
            self.on[PEERS].relation_changed,
            self.on.database_relation_created,
            self.on.database_relation_joined,
            self.on.database_relation_changed,
        ):
            self.framework.observe(event, self._on_reconcile)
        self.framework.observe(self.on.database_relation_broken, self._on_database_broken)
        self.framework.observe(self.on[PEERS].relation_broken, self._on_peer_broken)
        self.framework.observe(self.on.secret_changed, self._on_secret_changed)
        self.framework.observe(self.on.rotate_jwt_secret_action, self._on_rotate_jwt)
        self.framework.observe(self.on.ratings_pebble_check_failed, self._on_check)
        self.framework.observe(self.on.ratings_pebble_check_recovered, self._on_check)
        self.framework.observe(self.on.stop, self._on_stop)

    def _jwt_secret(self) -> ops.Secret | None:
        peer = self.model.get_relation(PEERS)
        if not peer:
            return None
        secret_id = peer.data[self.app].get(JWT_ID)
        if secret_id:
            # Never replace a published secret silently: doing so invalidates all JWTs.
            return self.model.get_secret(id=secret_id)
        if not self.unit.is_leader():
            return None
        try:
            secret = self.model.get_secret(label=JWT_LABEL)
        except ops.SecretNotFoundError:
            secret = self.app.add_secret(
                {"jwt-secret": base64.b64encode(secrets.token_bytes(48)).decode("ascii")},
                label=JWT_LABEL,
                description="Shared Ratings JWT signing key; all units of this app are owners",
            )
        # Recover the same labelled secret after a hook failure between creation/publication.
        secret_id = secret.id or secret.get_info().id
        peer.data[self.app][JWT_ID] = secret_id
        peer.data[self.app].setdefault(JWT_GENERATION, "0")
        return self.model.get_secret(id=secret_id)

    def _request_database(self) -> None:
        if not self.unit.is_leader():
            return
        for relation in self.model.relations["database"]:
            # Also initialise a relation that was created before this unit became leader.
            self.database.update_relation_data(
                relation.id,
                {
                    "database": DATABASE,
                    "requested-secrets": json.dumps(self.database.remote_secret_fields),
                },
            )

    def _configuration(self) -> tuple[dict[str, str], str | None, ops.StatusBase | None]:
        url = cast(str, self.config["snap-store-url"])
        try:
            parsed = urlsplit(url)
            _ = parsed.port
        except ValueError:
            raise ConfigurationError("snap-store-url has an invalid host or port") from None
        if (
            parsed.scheme not in {"http", "https"}
            or not parsed.hostname
            or parsed.username is not None
            or parsed.password is not None
            or parsed.query
            or parsed.fragment
            or not parsed.path.endswith("/")
            or any(char.isspace() for char in url)
        ):
            raise ConfigurationError(
                "snap-store-url must be an absolute HTTP(S) base URL ending in /"
            )
        rust_log = cast(str, self.config["rust-log"]).strip()
        if not rust_log or "\n" in rust_log or "\r" in rust_log:
            raise ConfigurationError("rust-log must be a non-empty, single-line Rust EnvFilter")
        secret = self._jwt_secret()
        if not secret:
            return (
                {},
                None,
                ops.WaitingStatus("Waiting for leader to publish the shared JWT secret"),
            )
        # Every unit owns this application secret. peek retrieves the latest revision
        # without --refresh (Juju rejects --refresh for owners, including peer units).
        jwt = secret.peek_content().get("jwt-secret")
        if not jwt:
            raise ConfigurationError(
                "Shared JWT secret has no jwt-secret value; restore the secret"
            )
        relations = self.model.relations["database"]
        if not relations:
            return {}, None, ops.BlockedStatus("Relate database to postgresql-k8s:database")
        if len(relations) != 1:
            raise ConfigurationError("Exactly one database relation is supported")
        # Use a fresh public data accessor: credentials/TLS come from Juju secrets
        # where negotiated. Avoid retaining a stale library cache across nested events.
        data = (
            DatabaseRequirerData(self.model, "database", DATABASE)
            .fetch_relation_data(
                [relations[0].id],
                ["username", "password", "endpoints", "database", "tls", "tls-ca"],
            )
            .get(relations[0].id, {})
        )
        if not all(data.get(key) for key in ("username", "password", "endpoints")):
            return (
                {},
                None,
                ops.WaitingStatus("Waiting for database credentials and primary endpoint"),
            )
        uri, ca = postgres_uri(data)
        return (
            {
                "APP_HOST": "0.0.0.0",
                "APP_PORT": str(PORT),
                "APP_POSTGRES_URI": uri,
                "APP_JWT_SECRET": jwt,
                "APP_SNAPCRAFT_IO_URI": url,
                "RUST_LOG": rust_log,
            },
            ca,
            None,
        )

    @staticmethod
    def _layer(environment: dict[str, str] | None = None) -> pebble.Layer:
        enabled = environment is not None
        return pebble.Layer(
            {
                "summary": "Ratings service",
                "services": {
                    SERVICE: {
                        "override": "replace",
                        "command": "/bin/ratings",
                        "startup": "enabled" if enabled else "disabled",
                        "environment": environment or {},
                        "on-failure": "restart" if enabled else "ignore",
                        "backoff-delay": "5s",
                        "backoff-limit": "1m",
                    }
                },
                "checks": {
                    READY_CHECK: {
                        "override": "replace",
                        "level": "ready",
                        "startup": "enabled" if enabled else "disabled",
                        "period": "5s",
                        "timeout": "2s",
                        "threshold": 1,
                        "tcp": {"host": "127.0.0.1", "port": PORT},
                    }
                },
            }
        )

    def _quiesce(self, status: ops.StatusBase) -> None:
        self.unit.set_ports()
        if not self.container.can_connect():
            self.unit.status = ops.WaitingStatus(
                "Waiting for Pebble to stop an unconfigured workload"
            )
            return
        plan = self.container.get_plan()
        if SERVICE in plan.services:
            self.container.stop(SERVICE)
        if READY_CHECK in plan.checks:
            self.container.stop_checks(READY_CHECK)
        # Override the image's startup-enabled service and erase stale environment.
        self.container.add_layer(SERVICE, self._layer(), combine=True)
        self.container.remove_path(CA_PATH, recursive=True)
        self.unit.status = status

    def _reconcile(
        self,
        *,
        unavailable: ops.StatusBase | None = None,
        refresh_secret: ops.Secret | None = None,
    ) -> None:
        try:
            if unavailable is not None:
                self._quiesce(unavailable)
                return
            try:
                if refresh_secret is not None:
                    refresh_secret.get_content(refresh=True)
                self._request_database()
                environment, ca, status = self._configuration()
            except ConfigurationError as exc:
                self._quiesce(ops.BlockedStatus(str(exc)))
                return
            except ops.SecretNotFoundError:
                self._quiesce(
                    ops.WaitingStatus("Waiting for Juju secret availability; restore if removed")
                )
                return
            except ops.ModelError as exc:
                if not any(
                    text in str(exc).lower() for text in ("permission denied", "not the leader")
                ):
                    raise
                self._quiesce(
                    ops.WaitingStatus("Waiting for secret access or Juju leadership to settle")
                )
                return
            if status is not None:
                self._quiesce(status)
                return
            if not self.container.can_connect():
                self.unit.set_ports()
                self.unit.status = ops.WaitingStatus("Waiting for Ratings Pebble container")
                return
            layer = self._layer(environment)
            plan = self.container.get_plan()
            changed = plan.services.get(SERVICE) != layer.services[SERVICE]
            if ca:
                try:
                    old_ca = self.container.pull(CA_PATH).read()
                except pebble.PathError as exc:
                    if exc.kind != "not-found":
                        raise
                    old_ca = None
                if old_ca != ca:
                    # A CA change must restart SQLx even if the URI path is unchanged.
                    changed = True
                    self.container.push(CA_PATH, ca, make_dirs=True, permissions=0o644)
            else:
                self.container.remove_path(CA_PATH, recursive=True)
            restarting = changed or (
                SERVICE in plan.services and not self.container.get_service(SERVICE).is_running()
            )
            if changed and SERVICE in plan.services:
                self.container.stop(SERVICE)
            if restarting and READY_CHECK in plan.checks:
                # A previous process's successful check must not make a new
                # process Active before the new listener is ready.
                self.container.stop_checks(READY_CHECK)
            self.container.add_layer(SERVICE, layer, combine=True)
            # Replan on every reconciliation, including recovery after a failed start.
            self.container.replan()
            self.unit.set_ports(PORT)
            self._health_status()
        except pebble.ConnectionError:
            self.unit.set_ports()
            self.unit.status = ops.WaitingStatus("Pebble unavailable; will retry on the next hook")
        except (pebble.ChangeError, pebble.TimeoutError):
            # Pebble task logs can contain workload output and secrets: do not log the exception.
            self.unit.set_ports()
            self.unit.status = ops.BlockedStatus(
                "Ratings Pebble operation failed; inspect workload logs"
            )

    def _health_status(self) -> None:
        services = self.container.get_services(SERVICE)
        if SERVICE not in services or not services[SERVICE].is_running():
            self.unit.status = ops.WaitingStatus("Waiting for Ratings service to run")
            return
        checks = self.container.get_checks(READY_CHECK)
        check = checks.get(READY_CHECK)
        if not check or check.status != pebble.CheckStatus.UP:
            self.unit.status = ops.WaitingStatus("Waiting for Ratings TCP readiness on port 8080")
            return
        if check.successes in (0, None):
            # A new check starts UP before its first success, without a recovery
            # event when it first succeeds. Confirm the shared-pod listener now
            # rather than waiting for update-status or trusting an untested check.
            try:
                with socket.create_connection(("127.0.0.1", PORT), timeout=2):
                    pass
            except OSError:
                self.unit.status = ops.WaitingStatus(
                    "Waiting for Ratings TCP readiness on port 8080"
                )
                return
        self.unit.status = ops.ActiveStatus()

    def _on_reconcile(self, event: ops.EventBase) -> None:
        self._reconcile()

    def _on_database_broken(self, event: ops.RelationBrokenEvent) -> None:
        # Relation data is no longer usable during relation-broken; do not fetch it.
        self._reconcile(unavailable=ops.BlockedStatus("Relate database to postgresql-k8s:database"))

    def _on_peer_broken(self, event: ops.RelationBrokenEvent) -> None:
        self._reconcile(unavailable=ops.WaitingStatus("Waiting for Ratings peer relation"))

    def _on_stop(self, event: ops.StopEvent) -> None:
        self._reconcile(unavailable=ops.MaintenanceStatus("Ratings stopped"))

    def _on_check(self, event: ops.PebbleCheckEvent) -> None:
        if event.info.name == READY_CHECK:
            self._reconcile()

    def _on_secret_changed(self, event: ops.SecretChangedEvent) -> None:
        peer = self.model.get_relation(PEERS)
        if peer and event.secret.id == peer.data[self.app].get(JWT_ID):
            self._reconcile()
            return
        # DatabaseRequires v0 does not emit credential-rotation events for PostgreSQL.
        # Refresh only a secret referenced by this database relation, then read through
        # the official data accessor. No credentials are copied into peer databags.
        for relation in self.model.relations["database"]:
            if not relation.app:
                continue
            if any(
                event.secret.id == value
                for key, value in relation.data[relation.app].items()
                if key.startswith("secret-")
            ):
                self._reconcile(refresh_secret=event.secret)
                return

    def _on_rotate_jwt(self, event: ops.ActionEvent) -> None:
        if not self.unit.is_leader():
            event.fail("Run rotate-jwt-secret on the leader unit")
            return
        peer = self.model.get_relation(PEERS)
        if not peer:
            event.fail("The Ratings peer relation is not available")
            return
        try:
            secret = self._jwt_secret()
            if secret is None:
                event.fail("The Ratings peer relation is not available")
                return
            secret.set_content(
                {"jwt-secret": base64.b64encode(secrets.token_bytes(48)).decode("ascii")}
            )
        except ops.SecretNotFoundError:
            event.fail("Shared JWT secret is unavailable; restore it before rotating")
            return
        # Secret owners do not receive secret-changed. Explicit peer churn makes
        # followers reload the latest revision; the leader reconciles immediately.
        peer.data[self.app][JWT_GENERATION] = secrets.token_hex(16)
        self._reconcile()
        event.set_results({"rotated": True})


if __name__ == "__main__":
    ops.main(RatingsCharm)
