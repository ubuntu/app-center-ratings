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
STARTUP = ["docker", "compose", "up", "--detach", "--build"]
WAIT = ["docker", "compose", "up", "--no-build", "--wait", "--wait-timeout", "600"]
PS = ["docker", "compose", "ps", "--all"]
LOGS = ["docker", "compose", "logs", "--no-color", "--tail=200"]
DOWN = ["docker", "compose", "down"]
DOCKER_FAILURES = {
    "startup": (STARTUP, 31),
    "readiness": (WAIT, 32),
    "ps": (PS, 33),
    "logs": (LOGS, 34),
    "down": (DOWN, 35),
}
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
if argv[0] == "python3" and argv[1:2] == ["-c"]:
    os.execv(sys.executable, [sys.executable, *sys.argv[1:]])
if argv[:3] == ["python3", "-m", "venv"]:
    directory = Path(argv[3]) / "bin"
    directory.mkdir(parents=True, exist_ok=True)
    for name in ("pip", "python", "ruff"):
        executable = directory / name
        executable.write_text(Path(__file__).read_text())
        executable.chmod(0o755)
keys = %r
event = {"argv": argv, "cwd": os.getcwd(), "env": {k: os.environ[k] for k in keys if k in os.environ}}
if argv == %r:
    event["stdin"] = sys.stdin.read()
with open(os.environ["MOCK_LOG"], "a") as log:
    log.write(json.dumps(event) + "\\n")
for phase, (command, status) in %r.items():
    matches = argv == command or (
        phase == "readiness" and argv[:-1] == command[:-1] and len(argv) == len(command)
    )
    if matches and phase in os.environ.get("MOCK_DOCKER_FAIL", "").split(","):
        print("mock docker failure: " + phase, file=sys.stderr)
        sys.exit(status)
if argv[:2] == ["multipass", "info"]:
    print(os.environ.get("MOCK_VM_INFO", "{}"))
if argv[:4] == ["sudo", "-n", "charmcraft", "pack"]:
    Path(os.environ["MOCK_CHARM_NAME"] + "_amd64.charm").write_bytes(b"mock charm")
if argv[0] == "cargo":
    phase = "db" if "db_tests" in argv else "integration"
    if os.environ.get("MOCK_CARGO_FAIL") == phase:
        print("mock cargo failure: " + phase, file=sys.stderr)
        sys.exit(23)
""" % ([*list(DB_ENV | INTEGRATION_ENV), "COMPOSE_FILE", "CARGO_TARGET_DIR"], CLEAR, DOCKER_FAILURES)


@unittest.skipUnless(JUST, "just is not installed")
class JustfileTests(unittest.TestCase):
    def setUp(self):
        self.root = Path(self.enterContext(tempfile.TemporaryDirectory(prefix="ratings just ")))
        for name in ("justfile", "justfile.vm"):
            shutil.copyfile(Path(__file__).resolve().parents[1] / name, self.root / name)
        (self.root / "charmcraft.yaml").write_text("name: app-center-ratings\n")
        self.app = self.root / "ratings_rock"
        (self.app / "tests").mkdir(parents=True)
        (self.app / "tests" / "clear-db.sql").write_text(SQL)
        for name in ("docker-compose.yml", "docker-compose.ci.yml"):
            (self.app / name).write_text("services: {}\n")
        self.bin = self.root / "bin"
        self.bin.mkdir()
        self.log = self.root / "commands.jsonl"
        # Restricted PATH prevents an accidental fallback to an installed external tool.
        (self.bin / "bash").symlink_to(shutil.which("bash"))
        for name in ("grep", "sed", "id", "mkdir", "install"):
            (self.bin / name).symlink_to(shutil.which(name))
        for name in ("docker", "cargo", "multipass", "docker-compose", "sudo", "rsync", "rockcraft", "python3"):
            executable = self.bin / name
            executable.write_text(f"#!{sys.executable}\n" + MOCK)
            executable.chmod(0o755)
        nested_just = self.bin / "just"
        nested_just.write_text(f'#!/bin/sh\nexec {shlex.quote(JUST)} "$@"\n')
        nested_just.chmod(0o755)
        self.env = {
            "PATH": str(self.bin), "HOME": os.environ.get("HOME", str(self.root)),
            "MOCK_LOG": str(self.log), "RATINGS_VM": "regression-vm",
            "MOCK_VM_INFO": json.dumps({"info": {"regression-vm": {"mounts": {}}}}),
            "MOCK_CHARM_NAME": "app-center-ratings",
        }

    def invoke(self, *args, failure=None, docker_failures=(), expected_status=0,
               justfile="justfile", cwd=None, variables=None):
        self.log.write_text("")
        env = dict(self.env)
        if failure:
            env["MOCK_CARGO_FAIL"] = failure
        if docker_failures:
            env["MOCK_DOCKER_FAIL"] = ",".join(docker_failures)
        settings = [item for key, value in (variables or {}).items() for item in ("--set", key, str(value))]
        result = subprocess.run(
            [JUST, "--justfile", str(self.root / justfile), *settings, *args],
            cwd=cwd or self.root, env=env, input="", text=True, capture_output=True, timeout=20,
        )
        self.output = result.stdout + result.stderr
        self.assertEqual(result.returncode, 23 if failure else expected_status, self.output)
        if failure:
            self.assertIn("mock cargo failure: " + failure, result.stderr)
        for phase in docker_failures:
            self.assertIn("mock docker failure: " + phase, result.stderr)
        return [json.loads(line) for line in self.log.read_text().splitlines()]

    def event(self, argv, env=None, **extra):
        cwd = self.root if argv[0] == "multipass" else self.app
        return {"argv": argv, "cwd": str(cwd), "env": env or {}, **extra}

    def integration(self, args=()):
        return [self.event(CLEAR, stdin=SQL), self.event(INTEGRATION + list(args), INTEGRATION_ENV)]

    def cleanup(self, failed=False):
        commands = [PS, LOGS, DOWN] if failed else [DOWN]
        return [self.event(command) for command in commands]

    def ci_events(self, events):
        compose_file = f"{self.app}/docker-compose.yml:{self.app}/docker-compose.ci.yml"
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

    def test_wait_for_server_uses_bounded_compose_readiness(self):
        for args, timeout in [((), "600"), (("45",), "45")]:
            with self.subTest(timeout=timeout):
                expected = [self.event(WAIT[:-1] + [timeout])]
                self.assertEqual(self.invoke("wait-for-server", *args), expected, self.output)

    def test_wait_for_server_preserves_compose_failure_status(self):
        expected = [self.event(WAIT[:-1] + ["45"])]
        events = self.invoke("wait-for-server", "45", docker_failures=("readiness",), expected_status=32)
        self.assertEqual(events, expected, self.output)

    def test_db_test_environment(self):
        self.assertEqual(self.invoke("db-test"), [self.event(DB, DB_ENV)], self.output)

    def test_integration_clears_database_and_preserves_variadic_arguments(self):
        args = ["filter with spaces", "--", "--nocapture", 'double"single\'', "$HOME; *", ""]
        self.assertEqual(self.invoke("integration-test", *args), self.integration(args), self.output)

    def test_test_all_runs_db_before_clear_and_integration_and_forwards_arguments(self):
        args = ["filter with spaces", "--", "*"]
        expected = [self.event(DB, DB_ENV), *self.integration(args)]
        self.assertEqual(self.invoke("test-all", *args), expected, self.output)

    def test_application_recipes_are_independent_of_invocation_directory(self):
        elsewhere = self.root / "elsewhere"
        elsewhere.mkdir()
        expected = [self.event(DB, DB_ENV), *self.integration()]
        self.assertEqual(self.invoke("test-all", cwd=elsewhere), expected, self.output)

    def test_vm_rust_recipes_use_application_directory_and_local_target(self):
        target = self.root / "vm-target"
        for recipe, command in (
            ("build", ["cargo", "build", "--locked", "--release"]),
            ("test", ["cargo", "test", "--locked", "--lib"]),
        ):
            with self.subTest(recipe=recipe):
                events = self.invoke(recipe, justfile="justfile.vm", variables={"target_dir": target})
                self.assertEqual(events, [self.event(command, {"CARGO_TARGET_DIR": str(target)})])

    def test_create_mounts_only_this_checkout_by_default(self):
        events = self.invoke("create")
        mounts = [e for e in events if e["argv"][:2] == ["multipass", "mount"]]
        self.assertEqual(mounts, [self.event([
            "multipass", "mount", "--type", "classic", str(self.root),
            "regression-vm:/home/ubuntu/app-center-ratings",
        ])])

    def test_default_operator_mount_verifies_root_charm_without_extra_mount(self):
        self.assertEqual(self.invoke("operator-mount"), [])
        (self.root / "charmcraft.yaml").write_text("name: unrelated\n")
        self.assertEqual(self.invoke("operator-mount", expected_status=1), [])

    def test_explicit_operator_override_mounts_and_is_forwarded_to_vm(self):
        operator = self.root / "other operator"
        operator.mkdir()
        (operator / "charmcraft.yaml").write_text("name: ratings\n")
        self.env["RATINGS_OPERATOR_REPO"] = str(operator)
        events = self.invoke("operator-mount")
        self.assertEqual(events[-1], self.event([
            "multipass", "mount", "--type", "classic", str(operator),
            "regression-vm:/home/ubuntu/ratings-operator",
        ]))
        events = self.invoke("vm", "charm-test", "filter with spaces")
        self.assertEqual(events[0]["argv"], [
            "multipass", "exec", "--working-directory", "/home/ubuntu/app-center-ratings",
            "regression-vm", "--", "env",
            "PATH=/home/ubuntu/.cargo/bin:/home/ubuntu/.local/bin:/snap/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin",
            "RATINGS_OPERATOR_REPO=/home/ubuntu/ratings-operator", "/snap/bin/just",
            "--justfile", "justfile.vm", "charm-test", "filter with spaces",
        ])

    def test_default_vm_forwarding_uses_root_operator(self):
        event = self.invoke("vm", "charm-pack")[0]
        self.assertIn("RATINGS_OPERATOR_REPO=/home/ubuntu/app-center-ratings", event["argv"])

    def test_vm_rock_syncs_only_application_and_packs_local_copy(self):
        source = self.root / "rock source"
        events = self.invoke("rock", justfile="justfile.vm", variables={"rock_source": source})
        sync, pack = events
        self.assertEqual(sync["argv"][:3], ["rsync", "-a", "--delete"])
        self.assertEqual(sync["argv"][-2:], [f"{self.app}/", f"{source}/"])
        self.assertIn("/target/", sync["argv"])
        self.assertIn("/mock_server/target/", sync["argv"])
        self.assertEqual(pack, {"argv": ["rockcraft", "pack"], "cwd": str(source), "env": {}})

    def test_vm_charm_pack_uses_root_metadata_and_excludes_application_and_caches(self):
        source, artifacts = self.root / "charm source", self.root / "artifacts"
        variables = {"charm_source": source, "rollout_dir": artifacts}
        events = self.invoke("charm-pack", justfile="justfile.vm", variables=variables)
        sync = next(e for e in events if e["argv"][0] == "rsync")
        self.assertEqual(sync["argv"][-2:], [f"{self.root}/", f"{source}/"])
        for pattern in ("/ratings_rock/", "/terraform/", "/dev/build/", "/parts/", "/target/",
                        ".venv/", "venv/", "virtualenv/", "*.charm", "*.rock"):
            self.assertIn(pattern, sync["argv"])
        pack = next(e for e in events if e["argv"][:4] == ["sudo", "-n", "charmcraft", "pack"])
        self.assertEqual(pack["cwd"], str(source))
        artifact = artifacts / "app-center-ratings_amd64.charm"
        self.assertEqual(artifact.read_bytes(), b"mock charm")

    def test_vm_charm_pack_preserves_explicit_legacy_override(self):
        operator = self.root / "other operator"
        operator.mkdir()
        (operator / "charmcraft.yaml").write_text("name: ratings\n")
        self.env["RATINGS_OPERATOR_REPO"] = str(operator)
        self.env["MOCK_CHARM_NAME"] = "ratings"
        source, artifacts = self.root / "charm source", self.root / "artifacts"
        events = self.invoke("charm-pack", justfile="justfile.vm",
                             variables={"charm_source": source, "rollout_dir": artifacts})
        sync = next(e for e in events if e["argv"][0] == "rsync")
        self.assertEqual(sync["argv"][-2:], [f"{operator}/", f"{source}/"])
        self.assertTrue((source / "ratings_amd64.charm").exists())
        self.assertEqual((artifacts / "app-center-ratings_amd64.charm").read_bytes(), b"mock charm")

    def test_vm_charm_test_uses_root_requirements_and_vm_local_venv(self):
        artifacts = self.root / "artifacts"
        events = self.invoke("charm-test", justfile="justfile.vm", variables={"rollout_dir": artifacts})
        expected = [
            ["python3", "-m", "venv", str(artifacts / "test-venv")],
            ["pip", "install", "-r", str(self.root / "requirements-dev.txt")],
            ["python", "-m", "pytest", "-q"],
            ["ruff", "check", "src", "tests"],
            ["ruff", "format", "--check", "src", "tests"],
        ]
        self.assertEqual(events, [{"argv": command, "cwd": str(self.root), "env": {}}
                                  for command in expected])

    def test_vm_charm_test_rejects_wrong_root_metadata_before_external_commands(self):
        (self.root / "charmcraft.yaml").write_text("name: ratings\n")
        self.assertEqual(self.invoke("charm-test", justfile="justfile.vm", expected_status=1), [])

    def test_vm_charm_pack_rejects_wrong_root_metadata_before_external_commands(self):
        (self.root / "charmcraft.yaml").write_text("name: ratings\n")
        self.assertEqual(self.invoke("charm-pack", justfile="justfile.vm", expected_status=1), [])

    def test_ci_startup_readiness_tests_and_cleanup_order(self):
        expected = [
            self.event(STARTUP), self.event(WAIT), self.event(DB, DB_ENV),
            *self.integration(), *self.cleanup(),
        ]
        self.assertEqual(self.invoke("ci-test"), self.ci_events(expected), self.output)
        self.assertEqual(self.invoke("up-detached"), [self.event(["docker", "compose", "up", "--detach"])])

    def test_ci_overrides_ambient_compose_file(self):
        self.env["COMPOSE_FILE"] = "unrelated-compose.yml"
        events = self.invoke("ci-test")
        self.assertEqual(events, self.ci_events(events), self.output)

    def test_ci_diagnoses_startup_and_readiness_failures_before_cleanup(self):
        for phase in ("startup", "readiness"):
            with self.subTest(phase=phase):
                expected = [self.event(STARTUP)]
                if phase == "readiness":
                    expected.append(self.event(WAIT))
                expected.extend(self.cleanup(failed=True))
                events = self.invoke(
                    "ci-test", docker_failures=(phase,), expected_status=DOCKER_FAILURES[phase][1],
                )
                self.assertEqual(events, self.ci_events(expected), self.output)

    def test_ci_cleans_up_after_either_cargo_phase_fails(self):
        startup = [self.event(STARTUP), self.event(WAIT)]
        for phase in ("db", "integration"):
            with self.subTest(phase=phase):
                expected = [*startup, self.event(DB, DB_ENV)]
                if phase == "integration":
                    expected.extend(self.integration())
                expected.extend(self.cleanup(failed=True))
                self.assertEqual(self.invoke("ci-test", failure=phase), self.ci_events(expected), self.output)

    def test_ci_preserves_failure_when_diagnostics_and_teardown_fail(self):
        for phase in ("readiness", "integration"):
            with self.subTest(phase=phase):
                expected = [self.event(STARTUP), self.event(WAIT)]
                failures = ("ps", "logs", "down")
                if phase == "integration":
                    expected.extend([self.event(DB, DB_ENV), *self.integration()])
                    events = self.invoke("ci-test", failure=phase, docker_failures=failures)
                else:
                    events = self.invoke("ci-test", docker_failures=(phase, *failures), expected_status=32)
                expected.extend(self.cleanup(failed=True))
                self.assertEqual(events, self.ci_events(expected), self.output)

    def test_ci_fails_when_teardown_fails_after_successful_tests(self):
        expected = [
            self.event(STARTUP), self.event(WAIT), self.event(DB, DB_ENV),
            *self.integration(), *self.cleanup(),
        ]
        events = self.invoke("ci-test", docker_failures=("down",), expected_status=35)
        self.assertEqual(events, self.ci_events(expected), self.output)


if __name__ == "__main__":
    unittest.main()
