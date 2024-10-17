FROM rust:1.81.0

RUN apt-get update && apt-get install -y protobuf-compiler netcat-openbsd
RUN cargo install cargo-watch

WORKDIR /app

ENTRYPOINT ["cargo watch -i 'tests/**' -x run"]
