"""Host ingress tests: all Multipass, SSH, and TLS commands are mocked."""

import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

import ingress_access as access
from juju_rollout import LabError

PUBLIC = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAITest ratings-lab-ingress"
REMOTE = access.remote


class AccessTests(unittest.TestCase):
    def setUp(self):
        self.root = Path(self.enterContext(tempfile.TemporaryDirectory()))
        self.enterContext(patch.object(subprocess, "run", side_effect=AssertionError("external command")))
        self.run = self.enterContext(patch.object(access, "run", side_effect=self.command))
        self.remote = self.enterContext(patch.object(access, "remote", side_effect=self.vm_command))
        self.context = self.enterContext(patch.object(access, "validate_ca"))

    def command(self, *args, **kwargs):
        if args[:2] == ("multipass", "info"):
            return json.dumps({"info": {"ratings-lab": {"state": "Running", "ipv4": ["192.168.64.5"]}}})
        if args[:2] == ("ssh-keygen", "-y"):
            return PUBLIC
        if args[0] == "ssh-keygen":
            Path(args[-1]).touch(mode=0o600)
        if args[:2] == ("openssl", "s_client"):
            return "ALPN protocol: h2\nVerify return code: 0 (ok)\n"

    def vm_command(self, vm, *args):
        if args[-1] == "endpoint":
            return json.dumps({"address": "10.20.30.40", "hostname": access.HOSTNAME})
        if args[0] == "cat":
            return PUBLIC
        if args[-1] == "ca":
            return "PUBLIC CA\n"

    def test_tunnel_pins_ssh_and_binds_only_loopback(self):
        access.tunnel("ratings-lab", 8443, self.root)
        args, kwargs = self.run.call_args
        self.assertEqual(args[0], "ssh")
        self.assertIn("127.0.0.1:8443:10.20.30.40:443", args)
        self.assertEqual(args[-1], "ubuntu@192.168.64.5")
        for flag in ("StrictHostKeyChecking=yes", "BatchMode=yes", "ExitOnForwardFailure=yes", "ForwardAgent=no"):
            self.assertIn(flag, args)
        self.assertIsNone(kwargs["timeout"])
        self.assertEqual(
            (self.root / "known_hosts").read_text(),
            "192.168.64.5 ssh-ed25519 " + PUBLIC.split()[1] + "\n",
        )
        self.assertNotIn("PRIVATE", repr(self.remote.call_args_list))
        self.run.reset_mock()
        access.tunnel("ratings-lab", 8443, self.root)
        self.assertFalse(any(c.args[:2] == ("ssh-keygen", "-q") for c in self.run.call_args_list))

    def test_bad_endpoint_or_vm_aborts_without_ssh(self):
        for reply in (
            {"address": "8.8.8.8", "hostname": access.HOSTNAME},
            {"address": "10.20.30.40", "hostname": "wrong.test"},
        ):
            self.remote.side_effect = None
            self.remote.return_value = json.dumps(reply)
            self.run.reset_mock()
            with self.assertRaises(LabError):
                access.tunnel("ratings-lab", 8443, self.root)
            self.assertFalse(any(c.args[0] == "ssh" for c in self.run.call_args_list))
        self.run.side_effect = None
        self.run.return_value = json.dumps({"info": {"ratings-lab": {"state": "Stopped"}}})
        with self.assertRaisesRegex(LabError, "not running"):
            access.tunnel("ratings-lab", 8443, self.root)

    def test_ssh_key_conflicts_and_authorization_failure(self):
        (self.root / "tunnel-ed25519.pub").touch()
        with self.assertRaisesRegex(LabError, "without private"):
            access.tunnel("ratings-lab", 8443, self.root)
        (self.root / "tunnel-ed25519.pub").unlink()
        self.remote.side_effect = [
            json.dumps({"address": "10.20.30.40", "hostname": access.HOSTNAME}),
            subprocess.CalledProcessError(1, ["multipass", "exec"]),
        ]
        with self.assertRaises(subprocess.CalledProcessError):
            access.tunnel("ratings-lab", 8443, self.root)
        self.assertFalse(any(c.args[0] == "ssh" for c in self.run.call_args_list))

    def test_authorize_preserves_other_keys_and_restricts_forwarding(self):
        ssh = self.root / ".ssh"
        ssh.mkdir()
        authorized = ssh / "authorized_keys"
        authorized.write_text("ssh-ed25519 OTHER unrelated\n")
        with (
            patch("pathlib.Path.home", return_value=self.root),
            patch("sys.argv", ["authorize", PUBLIC, "10.20.30.40"]),
        ):
            exec(compile(access.AUTHORIZE, "<authorize>", "exec"), {})
            original = authorized.read_bytes()
            exec(compile(access.AUTHORIZE, "<authorize>", "exec"), {})
        self.assertEqual(authorized.read_bytes(), original)
        text = authorized.read_text()
        self.assertTrue(text.startswith("ssh-ed25519 OTHER unrelated\n"))
        self.assertIn('restrict,port-forwarding,permitopen="10.20.30.40:443",command="/bin/false"', text)
        self.assertEqual(authorized.stat().st_mode & 0o777, 0o600)

    def test_authorize_refuses_unmanaged_key_and_symlink(self):
        ssh = self.root / ".ssh"
        ssh.mkdir()
        authorized = ssh / "authorized_keys"
        authorized.write_text(PUBLIC + "\n")
        for symlink in (False, True):
            if symlink:
                authorized.unlink()
                authorized.symlink_to(self.root / "other")
            with (
                patch("pathlib.Path.home", return_value=self.root),
                patch("sys.argv", ["authorize", PUBLIC, "10.20.30.40"]),
            ):
                with self.assertRaises(SystemExit):
                    exec(compile(access.AUTHORIZE, "<authorize>", "exec"), {})

    def test_ca_is_atomic_private_mode_and_failure_preserves_existing(self):
        access.export_ca("ratings-lab", self.root)
        path = self.root / "edge-ca.pem"
        self.assertEqual(path.read_text(), "PUBLIC CA\n")
        self.assertEqual(path.stat().st_mode & 0o777, 0o600)
        self.context.side_effect = LabError("malformed CA")
        with self.assertRaises(LabError):
            access.export_ca("ratings-lab", self.root)
        self.assertEqual(path.read_text(), "PUBLIC CA\n")
        self.remote.side_effect = subprocess.TimeoutExpired("multipass", 120)
        with self.assertRaises(subprocess.TimeoutExpired):
            access.export_ca("ratings-lab", self.root)
        self.assertEqual(path.read_text(), "PUBLIC CA\n")

    def test_tls_check_requires_ca_hostname_and_h2(self):
        (self.root / "edge-ca.pem").write_text("PUBLIC CA\n")
        access.tls_check(8443, self.root)
        args, kwargs = self.run.call_args
        self.assertIn("-verify_return_error", args)
        self.assertIn("-verify_hostname", args)
        self.assertIn("-servername", args)
        self.assertIn("-CAfile", args)
        self.assertEqual(args[-2:], ("-alpn", "h2"))
        self.assertEqual(kwargs["timeout"], 20)
        self.run.side_effect = None
        for reply in ("ALPN protocol: http/1.1\nVerify return code: 0 (ok)", "ALPN protocol: h2"):
            self.run.return_value = reply
            with self.assertRaises(LabError):
                access.tls_check(8443, self.root)
        self.run.side_effect = subprocess.CalledProcessError(1, ["openssl", "s_client"])
        with self.assertRaises(subprocess.CalledProcessError):
            access.tls_check(8443, self.root)

    def test_multipass_remote_commands_are_bounded(self):
        self.run.side_effect = None
        self.run.return_value = "result"
        self.assertEqual(REMOTE("ratings-lab", "true"), "result")
        self.run.assert_called_once_with(
            "multipass", "exec", "--no-map-working-directory", "ratings-lab", "--", "true",
            capture=True, timeout=120,
        )
        from juju_rollout import run as bounded_run
        with (
            self.assertRaises(LabError),
            patch.object(subprocess, "run", side_effect=subprocess.TimeoutExpired("multipass", 120)),
        ):
            bounded_run("multipass", "info", "ratings-lab", capture=True, timeout=120)


if __name__ == "__main__":
    unittest.main()
