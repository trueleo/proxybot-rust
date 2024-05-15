FROM rust:1.78.0-slim as builder

RUN rustup target add x86_64-unknown-linux-musl && \
    apt update && \
    apt install -y musl-tools musl-dev && \
    update-ca-certificates

COPY ./src ./src
COPY ./Cargo.lock .
COPY ./Cargo.toml .

RUN cargo build --target x86_64-unknown-linux-musl --release

FROM rust:1.78-alpine3.19

WORKDIR /app

COPY ./.env .

COPY --from=builder ./target/x86_64-unknown-linux-musl/release/proxybot .

ENTRYPOINT ["./proxybot"]
