FROM ubuntu:22.04

RUN apt-get update && apt-get upgrade -y && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY target/release/ratings /app/ratings

EXPOSE 8080
ENTRYPOINT ["/app/ratings"]
