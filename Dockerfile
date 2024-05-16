FROM rust:alpine as builder

RUN apk add musl-dev

RUN mkdir /app
WORKDIR /app

ARG RUSTFLAGS="-C target-feature=+crt-static"

COPY Cargo.toml /app
COPY Cargo.lock /app
COPY src /app/src

RUN cargo build --release --target=x86_64-unknown-linux-musl

RUN strip -s /app/target/x86_64-unknown-linux-musl/release/proxybot && \
    strip -R .comment -R .note -R .note.ABI-tag /app/target/x86_64-unknown-linux-musl/release/proxybot

FROM scratch
WORKDIR /app
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/proxybot /app/proxybot

VOLUME ["/app/userdata.db"]

EXPOSE 8080
CMD ["./proxybot"]