FROM rust:1.81.0

RUN apt-get update && apt-get install -y curl jq protobuf-compiler
RUN cargo install cargo-watch

WORKDIR /app

ENTRYPOINT ["cargo watch -x run"]
