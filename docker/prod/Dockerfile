FROM ubuntu:latest

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl-dev \
    jq \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY target/release/ratings /app/ratings

EXPOSE 443
ENTRYPOINT ["/app/ratings"]

