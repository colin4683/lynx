FROM rust:1.87-bookworm AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*
COPY lynx-agent ./lynx-agent
COPY lynx-proto ./lynx-proto
WORKDIR /app/lynx-agent
ENV SQLX_OFFLINE=true
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates tzdata procps && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/lynx-agent/target/release/lynx-agent /usr/local/bin/lynx-agent
COPY lynx-agent/certs ./certs
COPY lynx-agent/config.toml ./config.toml
ENV RUST_LOG=info
CMD ["lynx-agent"]