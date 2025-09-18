FROM rust:1.87-bookworm AS builder
WORKDIR /app
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*
COPY lynx-core ./lynx-core
COPY lynx-proto ./lynx-proto
WORKDIR /app/lynx-core
ENV SQLX_OFFLINE=true
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libssl-dev ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/lynx-core/target/release/lynx-core /usr/local/bin/lynx-core
COPY lynx-core/certs ./certs
ENV RUST_LOG=info
EXPOSE 50051
CMD ["lynx-core"]