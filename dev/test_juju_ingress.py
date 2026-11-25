"""Offline ingress topology and failure regression tests. Never contact a controller."""

import copy
import json
from pathlib import Path
import subprocess

import unittest
from unittest.mock import patch

import juju_ingress as ingress


def app(charm, revision=0, machine=False):
    value = {
        "charm-name": charm,
        "charm-rev": revision,
        "application-status": {"current": "active"},
        "units": {"unit/0": {"workload-status": {"current": "active"}, "juju-status": {"current": "idle"}}},
        "relations": {},
    }
    if not machine:
        value["scale"] = 1
    return value


class FakeJuju:
    """Use Juju 3.6's per-application JSON relations (no top-level relation array)."""

    def __init__(self):
        self.models = {
            ingress.MODEL: {"applications": {"ratings": app("app-center-ratings")}},
            ingress.EDGE_MODEL: {"applications": {}},
        }
        self.models[ingress.MODEL]["applications"]["ratings"]["endpoint-bindings"] = {"traefik-route": "alpha"}
        self.offers = {model: {} for model in self.models}
        self.configs = {}
        self.controllers = {ingress.CONTROLLER: {"cloud": "localhost"}}
        self.model_exists = True
        self.mutations = []
        self.calls = []

    def query(self, command, *args):
        model = args[args.index("--model") + 1] if "--model" in args else None
        if command == "show-cloud":
            return {"localhost": {"type": "lxd"}}
        if command == "controllers":
            return {"controllers": copy.deepcopy(self.controllers)}
        if command == "models":
            return {"models": [{"short-name": "ratings-edge"}] if self.model_exists else []}
        if command == "show-model":
            return {model: {"model-type": "iaas" if model == ingress.EDGE_MODEL else "caas",
                            "cloud": "localhost" if model == ingress.EDGE_MODEL else "k8s"}}
        if command == "status":
            return copy.deepcopy(self.models[model])
        if command == "offers":
            return copy.deepcopy(self.offers[model])
        if command == "config":
            name = args[-1]
            defaults = {
                key: {"value": None, "source": "default"}
                for key in ingress.CONFIG.get(name, {})
            }
            return {"settings": copy.deepcopy(self.configs.get((model, name), defaults))}
        raise AssertionError((command, args))

    def run(self, *args, **kwargs):
        self.calls.append((args, kwargs))
        if args[:2] == ("lxc", "info"):
            return "{}"
        assert args[0] == "juju", args
        self.mutations.append(args)
        command = args[1]
        if command == "bootstrap":
            self.controllers[ingress.CONTROLLER] = {"cloud": "localhost"}
            return
        model = args[args.index("--model") + 1] if "--model" in args else None
        if command == "add-model":
            self.model_exists = True
        elif command == "deploy":
            charm, name = args[4:6]
            revision = int(next(a.split("=", 1)[1] for a in args if a.startswith("--revision=")))
            self.models[model]["applications"][name] = app(charm, revision, model == ingress.EDGE_MODEL)
        elif command == "config":
            name = args[4]
            settings = self.query("config", "--model", model, name)["settings"]
            for setting in args[5:]:
                key, value = setting.split("=", 1)
                if value in ("true", "false"):
                    value = value == "true"
                settings[key] = {"value": value, "source": "user"}
            self.configs[model, name] = settings
        elif command == "offer":
            controller = args[args.index("--controller") + 1]
            source, name = args[-2:]
            short_model, rest = source.split(".")
            application, endpoint = rest.split(":")
            source_model = f"{controller}:{short_model}"
            interface = "certificate_transfer" if endpoint == "send-ca-cert" else "haproxy-route"
            self.offers[source_model][name] = {
                "application": application, "offer-url": f"admin/{short_model}.{name}",
                "endpoints": {endpoint: {"interface": interface}},
            }
        elif command == "consume":
            url, alias = args[-2:]
            self.models[model].setdefault("application-endpoints", {})[alias] = {"url": url}
        elif command == "integrate":
            left, right = args[-2:]
            name, endpoint = left.split(":")
            remote = right.split(":")[0]
            self.models[model]["applications"][name]["relations"][endpoint] = [{"related-application": remote}]
        else:
            raise AssertionError(args)


class IngressTests(unittest.TestCase):
    def setUp(self):
        self.fake = FakeJuju()
        self.enterContext(patch.object(subprocess, "run", side_effect=AssertionError("external command")))
        self.enterContext(patch.object(ingress, "run", side_effect=self.fake.run))
        self.query = self.enterContext(patch.object(ingress, "juju_json", side_effect=self.fake.query))

    def test_explicit_legacy_operator_name_remains_compatible(self):
        self.fake.models[ingress.MODEL]["applications"]["ratings"]["charm-name"] = "ratings"
        ingress.deploy()
        self.assertTrue(self.fake.mutations)

    def test_unrelated_ratings_charm_is_refused(self):
        self.fake.models[ingress.MODEL]["applications"]["ratings"]["charm-name"] = "unrelated"
        with self.assertRaises(ingress.LabError):
            ingress.deploy()
        self.assertEqual(self.fake.mutations, [])

    def test_exact_pinned_topology_and_noop_repeat(self):
        ingress.deploy()
        commands = self.fake.mutations
        deployments = [cmd for cmd in commands if cmd[1] == "deploy"]
        self.assertEqual(len(deployments), 5)
        for model, specs in ((ingress.MODEL, ingress.K8S_APPS), (ingress.EDGE_MODEL, ingress.EDGE_APPS)):
            for name, (charm, revision, channel, base, trust) in specs.items():
                cmd = next(c for c in deployments if c[3] == model and c[5] == name)
                self.assertEqual(cmd[4], charm)
                for flag in (
                    f"--revision={revision}", f"--channel={channel}",
                    f"--base=ubuntu@{base}", "--num-units=1",
                ):
                    self.assertIn(flag, cmd)
                self.assertEqual("--trust" in cmd, trust)
        integrations = [cmd[3:] for cmd in commands if cmd[1] == "integrate"]
        expected = [(ingress.MODEL, *pair) for pair in ingress.K8S_RELATIONS]
        expected += [(ingress.EDGE_MODEL, *pair) for pair in ingress.EDGE_RELATIONS]
        self.assertCountEqual(integrations, expected)
        consumes = [cmd[-2:] for cmd in commands if cmd[1] == "consume"]
        self.assertCountEqual(consumes, [
            ("ratings-lxd:admin/ratings-edge.ratings-edge", "edge"),
            ("ratings-k8s:admin/ratings-dev.ratings-traefik-ca", "traefik-ca"),
        ])
        settings = self.fake.configs[ingress.MODEL, "ingress-configurator"]
        self.assertEqual(settings["backend-protocol"]["value"], "https")
        self.assertEqual(settings["paths"]["value"], "/")
        self.assertEqual(settings["path-rewrite-expressions"]["value"], "")
        self.assertEqual(self.fake.configs[ingress.MODEL, "traefik"]["routing_mode"]["value"], "path")
        self.fake.mutations.clear()
        ingress.deploy()
        self.assertEqual(self.fake.mutations, [])

    def test_sidecar_requirement_and_wrong_charm_fail_before_mutation(self):
        for problem in ("endpoint", "charm", "revision"):
            with self.subTest(problem=problem):
                self.fake = FakeJuju()
                if problem == "endpoint":
                    self.fake.models[ingress.MODEL]["applications"]["ratings"]["endpoint-bindings"] = {}
                else:
                    self.fake.models[ingress.MODEL]["applications"]["traefik"] = app(
                        "unrelated" if problem == "charm" else "traefik-k8s", 999
                    )
                self.query.side_effect = self.fake.query
                with patch.object(ingress, "run", side_effect=self.fake.run), self.assertRaises(ingress.LabError):
                    ingress.deploy()
                self.assertEqual(self.fake.mutations, [])

    def test_query_errors_fail_closed(self):
        self.query.side_effect = subprocess.CalledProcessError(1, ["juju", "show-model"])
        with self.assertRaises(subprocess.CalledProcessError):
            ingress.deploy()
        self.assertEqual(self.fake.mutations, [])

    def test_relation_conflict_refused_in_real_status_shape(self):
        current = self.fake.models[ingress.MODEL]
        current["applications"]["ratings"]["relations"]["traefik-route"] = [{"related-application": "other"}]
        with self.assertRaisesRegex(ingress.LabError, "Conflicting relation"):
            ingress.deploy()
        self.assertEqual(self.fake.mutations, [])

    def test_wrong_controller_and_model_rejected(self):
        self.fake.controllers[ingress.CONTROLLER]["cloud"] = "production"
        with self.assertRaises(ingress.LabError):
            ingress.bootstrap()
        with self.assertRaises(ingress.LabError):
            ingress.edge_model()
        self.query.return_value = {"model": {"model-type": "caas", "cloud": "other"}}
        self.query.side_effect = None
        with self.assertRaises(ingress.LabError):
            ingress.check_model(ingress.MODEL, "caas", "k8s")
        self.assertEqual(self.fake.mutations, [])

    def test_bootstrap_is_separate_bounded_and_repeatable(self):
        self.fake.controllers.clear()
        ingress.bootstrap()
        commands = self.fake.calls
        self.assertEqual(commands[0][0], ("lxc", "info", "local:"))
        args, kwargs = commands[1]
        self.assertEqual(args[:4], ("juju", "bootstrap", "localhost", "ratings-lxd"))
        self.assertIn("--no-switch", args)
        self.assertIn("bootstrap-timeout=900", args)
        self.assertEqual(kwargs["timeout"], 960)
        self.fake.mutations.clear()
        ingress.bootstrap()
        self.assertEqual(self.fake.mutations, [])

    def test_machine_model_creation_is_explicit_local_and_repeatable(self):
        self.fake.model_exists = False
        ingress.edge_model()
        self.assertEqual(len(self.fake.mutations), 1)
        command = self.fake.mutations[0]
        self.assertEqual(command[:4], ("juju", "add-model", "--controller", "ratings-lxd"))
        self.assertIn("--no-switch", command)
        self.assertIn("localhost", command)
        self.assertIn("ratings-edge", command)
        self.fake.mutations.clear()
        ingress.edge_model()
        self.assertEqual(self.fake.mutations, [])

    def test_partial_deploy_failure_can_resume_without_refresh(self):
        def failing_run(*args, **kwargs):
            if args[1] == "deploy" and args[5] == "ingress-configurator":
                raise subprocess.CalledProcessError(1, args)
            return self.fake.run(*args, **kwargs)

        with patch.object(ingress, "run", side_effect=failing_run):
            with self.assertRaises(subprocess.CalledProcessError):
                ingress.deploy()
        self.assertIn("traefik", self.fake.models[ingress.MODEL]["applications"])
        self.fake.mutations.clear()
        ingress.deploy()
        self.assertFalse(any(
            c[1] == "refresh" or (c[1] == "deploy" and c[5] == "traefik")
            for c in self.fake.mutations
        ))

    def test_config_conflict_is_not_overwritten(self):
        self.fake.configs[ingress.MODEL, "ratings"] = {
            "external-hostname": {"value": "other.test", "source": "user"}
        }
        with self.assertRaisesRegex(ingress.LabError, "Conflicting ratings config"):
            ingress.configure(ingress.MODEL, ["ratings"])
        self.assertEqual(self.fake.mutations, [])

    def test_offer_and_consumer_collisions(self):
        for collision in ("application", "alias", "offer"):
            with self.subTest(collision=collision):
                fake = FakeJuju()
                self.query.side_effect = fake.query
                if collision == "application":
                    fake.models[ingress.MODEL]["applications"]["edge"] = app("unrelated")
                elif collision == "alias":
                    fake.models[ingress.MODEL]["application-endpoints"] = {"edge": {"url": "other:admin/prod.edge"}}
                else:
                    fake.offers[ingress.EDGE_MODEL]["ratings-edge"] = {"application": "unrelated"}
                with self.assertRaises(ingress.LabError):
                    ingress.offer(
                        ingress.EDGE_MODEL, "edge", "haproxy-route", "haproxy-route",
                        "ratings-edge", ingress.MODEL, "edge",
                    )
                self.assertEqual(self.fake.mutations, [])

    def test_machine_readiness_without_scale_and_hook_errors(self):
        ingress.deploy()
        with patch.object(ingress.time, "sleep", side_effect=AssertionError("unexpected sleep")):
            ingress.wait(1)
        edge = self.fake.models[ingress.EDGE_MODEL]["applications"]["edge"]
        edge["units"]["unit/0"]["juju-status"]["current"] = "error"
        with self.assertRaisesRegex(ingress.LabError, "hook error"):
            ingress.wait(1)

    def test_wait_timeout(self):
        with patch.object(ingress.time, "monotonic", side_effect=[0, 2]):
            with self.assertRaisesRegex(ingress.LabError, "timed out"):
                ingress.wait(1)

    def test_only_single_private_machine_edge_is_forwarded(self):
        ingress.deploy()
        units = self.fake.models[ingress.EDGE_MODEL]["applications"]["edge"]["units"]
        units["unit/0"]["public-address"] = "10.20.30.40"
        self.assertEqual(ingress.edge_address(), "10.20.30.40")
        for address in ("8.8.8.8", "127.0.0.1", "0.0.0.0", "::1"):
            units["unit/0"]["public-address"] = address
            with self.assertRaises(ingress.LabError):
                ingress.edge_address()
        units["unit/1"] = copy.deepcopy(units["unit/0"])
        with self.assertRaises(ingress.LabError):
            ingress.edge_address()

    def test_ca_action_is_public_only_and_bounded(self):
        self.query.side_effect = [
            {"edge": {"model-type": "iaas", "cloud": "localhost"}},
            {"applications": {}},
            {"edge-certificates/0": {"status": "completed", "results": {"ca-certificate": "PUBLIC"}}},
        ]
        with patch.object(ingress, "validate_ca") as validate:
            self.assertEqual(ingress.ca_certificate(), "PUBLIC\n")
        validate.assert_called_once_with("PUBLIC\n")
        self.assertIn("--wait=60s", self.query.call_args.args)
        self.assertIn("get-ca-certificate", self.query.call_args.args)

    def test_ca_rejects_action_dumps_keys_and_broken_certificates(self):
        for pem in ("", "{}", "-----BEGIN PRIVATE KEY-----\nAAA\n-----END PRIVATE KEY-----\n",
                    "-----BEGIN CERTIFICATE-----\nAAA\n-----END CERTIFICATE-----\n"):
            with self.subTest(pem=pem), self.assertRaises((ingress.LabError, ValueError, OSError)):
                ingress.validate_ca(pem)

    def test_default_rollout_has_not_gained_ingress_dependencies(self):
        root = Path(__file__).resolve().parent.parent
        vm = (root / "justfile.vm").read_text()
        line = next(line for line in vm.splitlines() if line.startswith("rollout:"))
        self.assertEqual(
            line,
            "rollout: postgres rock charm-pack ratings-image ratings-deploy ratings-observability ratings-wait",
        )


if __name__ == "__main__":
    unittest.main()
