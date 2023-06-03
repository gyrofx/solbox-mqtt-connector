FROM rust:slim-bullseye as builder

WORKDIR /app

RUN apt update && apt install -y openssl libssl-dev pkg-config

COPY src /app/src/
COPY Cargo.toml Cargo.lock /app/
RUN cargo build --release

FROM debian:bullseye-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/solbox-mqtt-connector /usr/local/bin/solbox-mqtt-connector

CMD ["solbox-mqtt-connector"]

