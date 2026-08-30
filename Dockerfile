FROM cgr.dev/chainguard/rust:latest AS builder

WORKDIR /work

ARG RUSTFLAGS="-C strip=symbols"

COPY --chown=nonroot:nonroot Cargo.toml Cargo.lock ./
COPY --chown=nonroot:nonroot src ./src

RUN cargo build --release
RUN mkdir /tmp/proxybot-data

FROM cgr.dev/chainguard/glibc-dynamic:latest

COPY --from=builder/work/target/release/proxybot /usr/local/bin/proxybot
COPY --from=builder /tmp/proxybot-data /data

VOLUME ["/data"]
EXPOSE 8080

CMD ["/usr/local/bin/proxybot"]
