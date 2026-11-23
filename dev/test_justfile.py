"""Run the real just against temporary fixtures; never use live Docker/Cargo/VMs."""

import json
import os
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import tempfile
import unittest

JUST = shutil.which("just") or shutil.which("just", path="/snap/bin")
DB = ["cargo", "test", "--lib", "--features", "db_tests"]
INTEGRATION = ["cargo", "test", "--test", "*"]
CLEAR = ["docker", "compose", "exec", "-T", "db", "psql", "-U", "postgres", "ratings"]
DB_ENV = {
    "APP_HOST": "0.0.0.0", "APP_PORT": "8080", "APP_JWT_SECRET": "deadbeef",
    "APP_SNAPCRAFT_IO_URI": "localhost:11111/",
    "APP_POSTGRES_URI": "postgresql://migration_user:strongpassword@localhost:5432/ratings",
}
INTEGRATION_ENV = {
    "APP_JWT_SECRET": "deadbeef", "MOCK_ADMIN_URL": "http://127.0.0.1:11111/__admin__/register-snap",
    "HOST": "0.0.0.0", "PORT": "8080",
}
SQL = "TRUNCATE TABLE regression_fixture;\n"
MOCK = """import json, os, sys
from pathlib import Path
argv = [Path(sys.argv[0]).name, *sys.argv[1:]]
keys = %r
event = {"argv": argv, "env": {k: os.environ[k] for k in keys if k in os.environ}}
if argv == %r:
    event["stdin"] = sys.stdin.read()
with open(os.environ["MOCK_LOG"], "a") as log:
    log.write(json.dumps(event) + "\\n")
if argv == ["docker", "compose", "ps"]:
    print("ratings running healthy")
if argv[0] == "cargo":
    phase = "db" if "db_tests" in argv else "integration"
    if os.environ.get("MOCK_CARGO_FAIL") == phase:
        print("mock cargo failure: " + phase, file=sys.stderr)
        sys.exit(23)
""" % ([*list(DB_ENV | INTEGRATION_ENV), "COMPOSE_FILE"], CLEAR)


@unittest.skipUnless(JUST, "just is not installed")
class JustfileTests(unittest.TestCase):
    def setUp(self):
        self.root = Path(self.enterContext(tempfile.TemporaryDirectory(prefix="ratings just ")))
        shutil.copyfile(Path(__file__).resolve().parents[1] / "justfile", self.root / "justfile")
        (self.root / "tests").mkdir()
        (self.root / "tests" / "clear-db.sql").write_text(SQL)
        self.bin = self.root / "bin"
        self.bin.mkdir()
        self.log = self.root / "commands.jsonl"
        # Restricted PATH prevents an accidental fallback to an installed external tool.
        for name in ("bash", "grep", "sleep"):
            (self.bin / name).symlink_to(shutil.which(name))
        for name in ("docker", "cargo", "multipass", "docker-compose"):
            executable = self.bin / name
            executable.write_text(f"#!{sys.executable}\n" + MOCK)
            executable.chmod(0o755)
        nested_just = self.bin / "just"
        nested_just.write_text(f'#!/bin/sh\nexec {shlex.quote(JUST)} "$@"\n')
        nested_just.chmod(0o755)
        self.env = {
            "PATH": str(self.bin), "HOME": os.environ.get("HOME", str(self.root)),
            "MOCK_LOG": str(self.log), "RATINGS_VM": "regression-vm",
        }

    def invoke(self, *args, failure=None):
        self.log.write_text("")
        env = dict(self.env)
        if failure:
            env["MOCK_CARGO_FAIL"] = failure
        result = subprocess.run(
            [JUST, "--justfile", str(self.root / "justfile"), *args],
            cwd=self.root, env=env, input="", text=True, capture_output=True, timeout=20,
        )
        self.output = result.stdout + result.stderr
        if failure:
            self.assertNotEqual(result.returncode, 0, self.output)
            self.assertIn("mock cargo failure: " + failure, self.output)
        else:
            self.assertEqual(result.returncode, 0, self.output)
        return [json.loads(line) for line in self.log.read_text().splitlines()]

    def event(self, argv, env=None, **extra):
        return {"argv": argv, "env": env or {}, **extra}

    def integration(self, args=()):
        return [self.event(CLEAR, stdin=SQL), self.event(INTEGRATION + list(args), INTEGRATION_ENV)]

    def ci_events(self, events):
        compose_file = f"{self.root}/docker-compose.yml:{self.root}/docker-compose.ci.yml"
        return [{**event, "env": {**event["env"], "COMPOSE_FILE": compose_file}} for event in events]

    def test_all_migrated_targets_exist(self):
        self.assertEqual(self.invoke("--summary"), [])
        expected = {
            "build", "docker-build", "compose-up", "up-detached", "down", "test", "db-test",
            "integration-test", "test-all", "wait-for-server", "ci-test", "clear-db-data",
            "rebuild-local", "rm-db", "rm-volumes", "db-shell", "ratings-shell", "up",
        }
        self.assertTrue(expected <= set(self.output.split()), self.output)

    def test_local_and_compose_commands_and_multipass_up(self):
        cases = {
            "build": [["cargo", "build", "--release"]],
            "test": [["cargo", "test", "--lib"]],
            "docker-build": [["docker", "build", "-f", "docker/prod/Dockerfile", "."]],
            "compose-up": [["docker", "compose", "up"]],
            "up-detached": [["docker", "compose", "up", "--detach"]],
            "down": [["docker", "compose", "down"]],
            "rebuild-local": [["docker", "compose", "build"]],
            "rm-db": [["docker", "compose", "rm", "db"], ["docker", "volume", "rm", "-f", "postgres_data"]],
            "rm-volumes": [["docker", "volume", "rm", "-f", name] for name in ("target-cache", "cargo-cache")],
            "db-shell": [["docker", "exec", "-it", "ratings-db", "psql", "-U", "postgres", "ratings"]],
            "ratings-shell": [["docker", "exec", "-it", "ratings", "bash"]],
            "up": [["multipass", "start", "regression-vm"]],
        }
        for recipe, commands in cases.items():
            with self.subTest(recipe=recipe):
                self.assertEqual(self.invoke(recipe), [self.event(argv) for argv in commands], self.output)

    def test_up_detached_forwards_compose_arguments(self):
        expected = [self.event(["docker", "compose", "up", "--detach", "--build", "--wait"])]
        self.assertEqual(self.invoke("up-detached", "--build", "--wait"), expected, self.output)

    def test_db_test_environment(self):
        self.assertEqual(self.invoke("db-test"), [self.event(DB, DB_ENV)], self.output)

    def test_integration_clears_database_and_preserves_variadic_arguments(self):
        args = ["filter with spaces", "--", "--nocapture", 'double"single\'', "$HOME; *", ""]
        self.assertEqual(self.invoke("integration-test", *args), self.integration(args), self.output)

    def test_test_all_runs_db_before_clear_and_integration_and_forwards_arguments(self):
        args = ["filter with spaces", "--", "*"]
        expected = [self.event(DB, DB_ENV), *self.integration(args)]
        self.assertEqual(self.invoke("test-all", *args), expected, self.output)

    def test_ci_startup_readiness_tests_and_cleanup_order(self):
        expected = [
            self.event(["docker", "compose", "up", "--detach", "--build"]),
            self.event(["docker", "compose", "ps"]), self.event(DB, DB_ENV),
            *self.integration(), self.event(["docker", "compose", "down"]),
        ]
        self.assertEqual(self.invoke("ci-test"), self.ci_events(expected), self.output)
        self.assertIn("ratings running healthy", self.output)
        self.assertEqual(self.invoke("up-detached"), [self.event(["docker", "compose", "up", "--detach"])])

    def test_ci_overrides_ambient_compose_file(self):
        self.env["COMPOSE_FILE"] = "unrelated-compose.yml"
        events = self.invoke("ci-test")
        self.assertEqual(events, self.ci_events(events), self.output)

    def test_ci_cleans_up_after_either_cargo_phase_fails(self):
        startup = [self.event(["docker", "compose", "up", "--detach", "--build"]), self.event(["docker", "compose", "ps"])]
        for phase in ("db", "integration"):
            with self.subTest(phase=phase):
                expected = [*startup, self.event(DB, DB_ENV)]
                if phase == "integration":
                    expected.extend(self.integration())
                expected.append(self.event(["docker", "compose", "down"]))
                self.assertEqual(self.invoke("ci-test", failure=phase), self.ci_events(expected), self.output)


if __name__ == "__main__":
    unittest.main()
