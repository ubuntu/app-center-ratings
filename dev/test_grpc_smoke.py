"""Offline channel-selection checks; do not install gRPC or invoke a real RPC."""

import importlib.util
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import MagicMock, patch


class SmokeTransportTests(unittest.TestCase):
    def setUp(self):
        self.grpc = MagicMock()
        self.tools = MagicMock()
        self.tools.__file__ = __file__
        modules = {
            "grpc": self.grpc,
            "grpc_tools": self.tools,
            "grpc_tools.protoc": self.tools.protoc,
            "google": MagicMock(),
            "google.protobuf": MagicMock(),
            "google.protobuf.empty_pb2": MagicMock(),
        }
        self.enterContext(patch.dict(sys.modules, modules))
        path = Path(__file__).with_name("grpc_smoke.py")
        spec = importlib.util.spec_from_file_location("smoke_under_test", path)
        self.smoke = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(self.smoke)

    def test_existing_plaintext_mode_is_unchanged(self):
        self.smoke.connect("127.0.0.1:8080")
        self.grpc.insecure_channel.assert_called_once_with("127.0.0.1:8080")
        self.grpc.secure_channel.assert_not_called()

    def test_tls_channel_preserves_hostname_without_authority_override(self):
        credentials = object()
        self.smoke.connect("ratings.localhost:8443", credentials)
        self.grpc.secure_channel.assert_called_once_with("ratings.localhost:8443", credentials)
        self.grpc.insecure_channel.assert_not_called()

    def test_proto_codegen_uses_application_subdirectory(self):
        self.tools.protoc.main.return_value = 42
        with patch.object(sys, "argv", ["grpc_smoke.py"]):
            self.assertEqual(self.smoke.main(), 42)
        args = self.tools.protoc.main.call_args.args[0]
        proto = Path(__file__).resolve().parents[1] / "ratings_rock" / "proto"
        self.assertIn(f"-I{proto}", args)
        self.assertEqual(args[-3:], [
            str(proto / f"ratings_features_{name}.proto") for name in ("user", "common", "app")
        ])
        self.grpc.insecure_channel.assert_not_called()

    def test_ca_file_is_passed_as_tls_roots(self):
        with tempfile.TemporaryDirectory() as directory:
            ca = Path(directory) / "edge-ca.pem"
            ca.write_bytes(b"public CA fixture")
            # Stop before code generation; only exercise CLI and TLS-root loading.
            self.tools.protoc.main.return_value = 42
            with patch.object(sys, "argv", ["grpc_smoke.py", "--ca-file", str(ca)]):
                self.assertEqual(self.smoke.main(), 42)
        self.grpc.ssl_channel_credentials.assert_called_once_with(b"public CA fixture")
        self.grpc.insecure_channel.assert_not_called()


if __name__ == "__main__":
    unittest.main()
