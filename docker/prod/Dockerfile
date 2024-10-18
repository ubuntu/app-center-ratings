FROM ubuntu:latest

RUN DEBIAN_FRONTEND=noninteractive apt-get update && apt-get upgrade -y && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY target/release/ratings /app/ratings
COPY sql/migrations /app/sql/migrations

EXPOSE 443
ENTRYPOINT ["/app/ratings"]
