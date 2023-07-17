# HACKING

This document will help you get started with code contributions to Ratings.

## Required dependencies

* rust and cargo >= 1.69
* protobuf-compiler >= 3.21
* docker >= 24.04
* docker-compose >= 2.18

To get setup on Ubuntu:

```shell
# Install the Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
source "$HOME/.cargo/env"

# Install build-time dependencies
sudo apt update
sudo apt install -y git gcc libssl-dev pkg-config
sudo snap install --classic protobuf

# (Optional) Install rockcraft for building the OCI image
sudo snap install rockcraft --classic --channel edge

# (Optional) Install Docker for running the OCI image
curl -fsSL https://get.docker.com -o /tmp/get-docker.sh
sh /tmp/get-docker.sh
```

## Building and running the binaries

To build and run the binary during development:

```shell
# Setup a .env file
# You'll need to adjust the values in the resulting .env to point to a suitable PostgreSQL instance.
cp example.env .env

# Build and run
cargo run
```

To _just_ build the binary you can run `cargo build --release`. The result will be placed at
`./target/release/ratings`.

## Building and running the OCI image

The OCI image for Ratings is built using [`rockcraft`](https://github.com/canonical/rockcraft).

You can get started like so:

```shell
# Build the image
rockcraft --verbose

# Grab the version from the rockcraft.yaml
version="$(grep -Po "^version: \K.+" rockcraft.yaml)"

# Copy the image from the ROCK archive to the local docker daemon
sudo /snap/rockcraft/current/bin/skopeo \
    --insecure-policy \
    copy oci-archive:"ratings_${version}_amd64.rock" docker-daemon:"ratings:${version}"

# Run the service - replace the postgres var with valid database credentials
docker run --rm \
    -e POSTGRES="postgresql://user:password@localhost:5432/ratings" \
    -e JWT_SECRET="deadbeef" \
    -p 18080:18080 \
    "ratings:${version}"
```
