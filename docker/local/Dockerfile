FROM rust:1

RUN apt-get update && apt-get install -y protobuf-compiler netcat-openbsd
RUN cargo install cargo-watch --locked

WORKDIR /app

ENTRYPOINT ["cargo watch -i 'tests/**' -x run"]
