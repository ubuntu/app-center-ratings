FROM rust:1-bookworm

RUN apt-get update \
    && apt-get install -y --no-install-recommends libssl-dev pkg-config protobuf-compiler netcat-openbsd \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY rust-toolchain.toml ./
# Install the named channel required by the repo, not just the image's numeric toolchain.
RUN rustup toolchain install && rustup default stable

ENTRYPOINT ["cargo", "run", "--locked"]
