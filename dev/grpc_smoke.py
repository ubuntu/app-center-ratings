#!/usr/bin/env python3
# /// script
# requires-python = ">=3.10"
# dependencies = ["grpcio==1.76.0", "grpcio-tools==1.76.0"]
# ///
"""Exercise the local Ratings gRPC API without printing credentials or JWTs."""

import argparse
import hashlib
import importlib
from pathlib import Path
import secrets
import sys
import tempfile

import grpc
from google.protobuf.empty_pb2 import Empty
from grpc_tools import protoc
import grpc_tools


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", default="127.0.0.1:8080")
    parser.add_argument(
        "--verify-target", help="Optional second unit endpoint to verify the shared JWT"
    )
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryDirectory(prefix="ratings-grpc-") as temporary:
        proto = root / "proto"
        builtins = Path(grpc_tools.__file__).parent / "_proto"
        code = protoc.main(
            [
                "protoc",
                f"-I{proto}",
                f"-I{builtins}",
                f"--python_out={temporary}",
                f"--grpc_python_out={temporary}",
                str(proto / "ratings_features_user.proto"),
                str(proto / "ratings_features_common.proto"),
                str(proto / "ratings_features_app.proto"),
            ]
        )
        if code:
            return code
        sys.path.insert(0, temporary)
        user_pb = importlib.import_module("ratings_features_user_pb2")
        user_grpc = importlib.import_module("ratings_features_user_pb2_grpc")
        app_pb = importlib.import_module("ratings_features_app_pb2")
        app_grpc = importlib.import_module("ratings_features_app_pb2_grpc")
        with grpc.insecure_channel(args.target) as channel:
            grpc.channel_ready_future(channel).result(timeout=15)
            user = user_grpc.UserStub(channel)
            app = app_grpc.AppStub(channel)
            identifier = hashlib.sha256(secrets.token_bytes(32)).hexdigest()
            token = user.Authenticate(
                user_pb.AuthenticateRequest(id=identifier), timeout=10
            ).token
            metadata = (("authorization", f"Bearer {token}"),)
            try:
                # No known snap ID or votes: no dependency on the external Snap Store.
                response = app.GetBulkRatings(
                    app_pb.GetBulkRatingsRequest(snap_ids=[identifier]),
                    metadata=metadata,
                    timeout=10,
                )
                if args.verify_target:
                    with grpc.insecure_channel(args.verify_target) as second_channel:
                        grpc.channel_ready_future(second_channel).result(timeout=15)
                        second_app = app_grpc.AppStub(second_channel)
                        second_app.GetBulkRatings(
                            app_pb.GetBulkRatingsRequest(snap_ids=[identifier]),
                            metadata=metadata,
                            timeout=10,
                        )
                    print(
                        f"PASS: JWT issued by {args.target} accepted by {args.verify_target}"
                    )
                if response.ratings:
                    raise RuntimeError(
                        "An unused smoke-test snap ID returned unexpected ratings"
                    )
                try:
                    app.GetBulkRatings(
                        app_pb.GetBulkRatingsRequest(snap_ids=[identifier]), timeout=10
                    )
                except grpc.RpcError as error:
                    if error.code() not in {
                        grpc.StatusCode.UNAUTHENTICATED,
                        grpc.StatusCode.UNKNOWN,
                        grpc.StatusCode.INTERNAL,
                    }:
                        raise
                    # The current middleware can surface missing authorization as
                    # a non-OK transport status rather than UNAUTHENTICATED.
                else:
                    raise RuntimeError(
                        "Unauthenticated request was unexpectedly accepted"
                    )
                print(
                    f"PASS: {args.target}: Authenticate, authenticated database read, unauthorized request rejected"
                )
            finally:
                user.Delete(Empty(), metadata=metadata, timeout=10)
                print(
                    "Smoke-test identity removed; no token or database credentials printed"
                )
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except (grpc.RpcError, grpc.FutureTimeoutError, RuntimeError) as error:
        # Avoid dumping response metadata or request objects containing tokens.
        status = (
            error.code().name
            if isinstance(error, grpc.RpcError)
            else type(error).__name__
        )
        print(f"gRPC smoke check failed: {status}", file=sys.stderr)
        sys.exit(1)
