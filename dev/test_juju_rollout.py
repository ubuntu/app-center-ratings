"""Offline regression tests; never invoke Juju, a registry, or a VM."""

import hashlib
import json
import subprocess
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import juju_rollout as rollout

MODEL = "controller:ratings-dev"
COS = "controller:cos-lite"
DIGEST = "sha256:" + "a" * 64
IMAGE = f"{rollout.REGISTRY}/ratings@{DIGEST}"
URL = "admin/cos-lite.ratings-logs"


def status():
    return json.loads("""{"applications": {
        "ratings": {"charm-name": "ratings", "scale": 1,
            "application-status": {"current": "active"}, "relations": {}, "units": {
                "ratings/0": {"workload-status": {"current": "active"},
                              "juju-status": {"current": "idle"}}}},
        "postgresql": {"charm-name": "postgresql-k8s", "scale": 1,
            "application-status": {"current": "active"}, "relations": {}, "units": {
                "postgresql/0": {"workload-status": {"current": "active"},
                                 "juju-status": {"current": "idle"}}}}
    }}""")


def unit(current, app="ratings"):
    return current["applications"][app]["units"][f"{app}/0"]


class RolloutTests(unittest.TestCase):
    def setUp(self):
        self.root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        self.charm, self.rock = self.root / "ratings.charm", self.root / "ratings.rock"
        self.charm.touch()
        self.rock.write_bytes(b"rock")
        self.image = self.root / "image.json"
        self.image.write_text(json.dumps({"image": IMAGE}))
        self.digest, self.responses = DIGEST, iter(())
        guard = patch.object(
            rollout.subprocess, "run", side_effect=AssertionError("external command")
        )
        self.enterContext(guard)
        self.run = self.enterContext(
            patch.object(rollout, "run", side_effect=self.command)
        )
        self.sleep = self.enterContext(patch.object(rollout.time, "sleep"))
        self.clock = self.enterContext(patch.object(rollout.time, "monotonic"))
        self.clock.side_effect = [0, 0, 1, 11]

    def command(self, *args, **kwargs):
        if args[:2] == ("skopeo", "inspect"):
            return self.digest + "\n"
        if args[0] == "juju" and kwargs.get("capture"):
            response = next(self.responses)
            if isinstance(response, Exception):
                raise response
            return response if isinstance(response, str) else json.dumps(response)

    def replies(self, *responses):
        self.responses = iter(responses)
        self.run.reset_mock()

    def mutations(self):
        return [
            c.args
            for c in self.run.call_args_list
            if c.args[0] == "juju" and c.args[1] not in ("status", "offers")
        ]

    def test_deploy_then_refresh_does_not_duplicate_relation(self):
        absent, deployed, connected = status(), status(), status()
        del absent["applications"]["ratings"]
        connected["applications"]["ratings"]["relations"] = {"database": ["postgresql"]}
        self.replies(absent, deployed, connected, connected)
        rollout.deploy(self.charm, self.root, MODEL)
        rollout.deploy(self.charm, self.root, MODEL)
        commands = self.mutations()
        self.assertEqual(
            [cmd[1] for cmd in commands], ["deploy", "integrate", "refresh"]
        )
        self.assertEqual(commands[1][-2:], ("ratings:database", "postgresql:database"))
        for command in (commands[0], commands[2]):
            self.assertIn(f"ratings-image={IMAGE}", command)
            self.assertIn(self.charm, command)
            self.assertIn(MODEL, command)
        self.assertIn("--num-units=1", commands[0])

    def test_wrong_application_or_provider_refused_before_mutation(self):
        for problem in ("missing", "postgresql", "ratings", "database"):
            with self.subTest(problem=problem):
                current = status()
                if problem == "missing":
                    del current["applications"]["postgresql"]
                elif problem == "database":
                    current["applications"]["ratings"]["relations"] = {
                        "database": ["other-db"]
                    }
                else:
                    current["applications"][problem]["charm-name"] = "unrelated"
                self.replies(current, current)
                with self.assertRaises(rollout.LabError):
                    rollout.deploy(self.charm, self.root, MODEL)
                self.assertEqual(self.mutations(), [])

    def test_query_errors_do_not_mutate(self):
        error = subprocess.CalledProcessError(1, ["juju", "status"])
        for response in (error, "not JSON"):
            for action in (
                lambda: rollout.deploy(self.charm, self.root, MODEL),
                lambda: rollout.observability("controller", MODEL, COS),
            ):
                with self.subTest(response=response, action=action):
                    self.replies(response)
                    with self.assertRaises((subprocess.CalledProcessError, ValueError)):
                        action()
                    self.assertEqual(self.mutations(), [])

    def test_invalid_images_rejected_without_commands(self):
        for image in (
            "example.com/ratings@" + DIGEST,
            IMAGE.replace("@", ":"),
            IMAGE[:-1],
        ):
            with self.subTest(image=image):
                self.image.write_text(json.dumps({"image": image}))
                with self.assertRaises(rollout.LabError):
                    rollout.deploy(self.charm, self.root, MODEL)
                self.run.assert_not_called()

    def test_publish_records_manifest_digest_not_archive_checksum(self):
        rollout.publish(self.rock, self.root)
        checksum = hashlib.sha256(b"rock").hexdigest()
        expected = {"image": IMAGE, "rock": str(self.rock), "sha256": checksum}
        self.assertEqual(json.loads(self.image.read_text()), expected)
        self.assertEqual(self.image.stat().st_mode & 0o777, 0o600)
        copy = self.run.call_args_list[0].args
        self.assertIn("--preserve-digests", copy)
        self.assertIn(f"docker://{rollout.REGISTRY}/ratings:{checksum[:20]}", copy)

    def test_invalid_registry_digest_preserves_image_state(self):
        original = self.image.read_bytes()
        for digest in ("latest", "sha256:" + "a" * 63, "sha256:" + "A" * 64):
            with self.subTest(digest=digest):
                self.digest = digest
                with self.assertRaises(rollout.LabError):
                    rollout.publish(self.rock, self.root)
                self.assertEqual(self.image.read_bytes(), original)

    def test_related_uses_juju_relation_objects_and_legacy_name_lists(self):
        current = status()
        self.assertFalse(rollout.related(current, "ratings", "database", "postgresql"))
        current["applications"]["ratings"]["relations"] = {
            "database": ["other-db", "postgresql"]
        }
        self.assertTrue(rollout.related(current, "ratings", "database", "postgresql"))
        self.assertFalse(rollout.related(current, "ratings", "database", "missing"))
        current["applications"]["ratings"]["relations"] = {
            "database": [
                {
                    "related-application": "postgresql",
                    "interface": "postgresql_client",
                    "scope": "global",
                }
            ]
        }
        self.assertTrue(rollout.related(current, "ratings", "database", "postgresql"))

    def observability_replies(
        self, existing=False, offer_app="loki", alias_url=URL, ingress=False
    ):
        current, connected = status(), status()
        connected["application-endpoints"] = {"ratings-loki": {"url": alias_url}}
        cos = {"applications": {"loki": {"charm-name": "loki-k8s"}}}
        if ingress:
            cos["applications"]["loki"]["relations"] = {
                "ingress": [
                    {
                        "related-application": "traefik",
                        "interface": "ingress_per_unit",
                        "scope": "global",
                    }
                ]
            }
        offers = json.loads("""{"ratings-logs": {"application": "loki",
            "offer-name": "ratings-logs", "offer-url": "admin/cos-lite.ratings-logs",
            "endpoints": {"logging": {"interface": "loki_push_api", "role": "provider"}}}}""")
        offers["ratings-logs"]["application"] = offer_app
        if existing:
            connected["applications"]["ratings"]["relations"] = {
                "logging": ["ratings-loki"]
            }
            self.replies(connected, cos, offers)
        else:
            self.replies(current, cos, {}, offers, connected)

    def test_observability_offers_consumes_and_integrates(self):
        self.observability_replies()
        rollout.observability("controller", MODEL, COS)
        commands = self.mutations()
        self.assertEqual(
            [cmd[1] for cmd in commands], ["offer", "consume", "integrate"]
        )
        self.assertEqual(commands[0][-2:], ("cos-lite.loki:logging", "ratings-logs"))
        self.assertEqual(commands[1][-2:], (f"controller:{URL}", "ratings-loki"))
        self.assertEqual(commands[2][-2:], ("ratings:logging", "ratings-loki:logging"))

    def test_observability_repeat_is_noop_for_both_url_forms(self):
        for url in (URL, f"controller:{URL}"):
            with self.subTest(url=url):
                self.observability_replies(existing=True, alias_url=url)
                rollout.observability("controller", MODEL, COS)
                self.assertEqual(self.mutations(), [])

    def test_observability_uses_loki_internal_endpoint_without_removing_other_routes(
        self,
    ):
        self.observability_replies(existing=True, ingress=True)
        rollout.observability("controller", MODEL, COS)
        self.assertEqual(
            self.mutations(),
            [
                (
                    "juju",
                    "remove-relation",
                    "--model",
                    COS,
                    "loki:ingress",
                    "traefik:ingress-per-unit",
                )
            ],
        )

    def test_observability_rejects_unrelated_offer_or_alias(self):
        for app, url in (("other-loki", URL), ("loki", "admin/other.logs")):
            with self.subTest(app=app, url=url):
                self.observability_replies(existing=True, offer_app=app, alias_url=url)
                with self.assertRaises(rollout.LabError):
                    rollout.observability("controller", MODEL, COS)
                self.assertEqual(self.mutations(), [])

    def test_managed_write_preserves_unmanaged_files_and_mode(self):
        self.image.chmod(0o640)
        original = self.image.read_bytes()
        with self.assertRaises(rollout.LabError):
            rollout.managed_write(self.image, rollout.MARKER + "replacement\n")
        self.assertEqual(self.image.read_bytes(), original)
        self.assertEqual(self.image.stat().st_mode & 0o777, 0o640)

    def test_managed_write_is_atomic_and_idempotent(self):
        path, text = (
            self.root / "hosts.toml",
            rollout.MARKER + "registry configuration\n",
        )
        with patch.object(rollout.os, "replace", wraps=rollout.os.replace) as replace:
            self.assertTrue(rollout.managed_write(path, text, 0o640))
            self.assertEqual(path.stat().st_mode & 0o777, 0o640)
            self.assertFalse(rollout.managed_write(path, text, 0o640))
            self.assertTrue(rollout.managed_write(path, text + "updated\n", 0o600))
        self.assertEqual(replace.call_count, 2)
        for replacement in replace.call_args_list:
            source, destination = map(Path, replacement.args)
            self.assertEqual(source.parent, destination.parent)
            self.assertNotEqual(source, destination)
        self.assertEqual(path.read_text(), text + "updated\n")
        self.assertEqual(path.stat().st_mode & 0o777, 0o600)

    def test_failed_atomic_replace_preserves_original_and_cleans_up(self):
        original = self.image.read_bytes()
        with patch.object(rollout.os, "replace", side_effect=OSError("replace failed")):
            with self.assertRaises(OSError):
                rollout.atomic_write(self.image, "replacement")
        self.assertEqual(self.image.read_bytes(), original)
        self.assertEqual(list(self.root.glob("image.json.*")), [])

    def test_health_requires_present_active_idle_units(self):
        for workload, agent in (
            ("active", "idle"),
            ("waiting", "idle"),
            ("active", "executing"),
        ):
            with self.subTest(workload=workload, agent=agent):
                current = status()
                unit(current)["workload-status"]["current"] = workload
                unit(current)["juju-status"]["current"] = agent
                ready = workload == "active" and agent == "idle"
                self.assertEqual(rollout.health(current, "ratings")[0], ready)
        current["applications"]["ratings"]["units"] = {}
        self.assertFalse(rollout.health(current, "ratings")[0])
        self.assertFalse(rollout.health(current, "missing")[0])

    def test_wait_checks_provider_units_despite_stale_active_app_status(self):
        stale = status()
        unit(stale, "postgresql")["workload-status"]["current"] = "waiting"
        self.replies(stale, status())
        rollout.wait(MODEL, 10)
        self.sleep.assert_called_once_with(5)

    def test_wait_hook_errors_stop_immediately(self):
        for app in ("ratings", "postgresql"):
            for field in ("workload-status", "juju-status"):
                with self.subTest(app=app, field=field):
                    self.clock.side_effect = [0, 0, 11]
                    current = status()
                    unit(current, app)[field]["current"] = "error"
                    self.replies(current)
                    with self.assertRaisesRegex(
                        rollout.LabError, f"{app}/0.*hook error"
                    ):
                        rollout.wait(MODEL, 10)
                    self.sleep.assert_not_called()
