FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder 
COPY --from=planner /app/recipe.json recipe.json
RUN rustup component add rustfmt
RUN apt-get update \
 && DEBIAN_FRONTEND=noninteractive \
    apt-get install --no-install-recommends --assume-yes \
      protobuf-compiler; \
    apt-get install --no-install-recommends --assume-yes \
      wget
RUN wget https://github.com/fullstorydev/grpcurl/releases/download/v1.8.9/grpcurl_1.8.9_linux_x86_64.tar.gz
RUN tar -xvzf grpcurl_1.8.9_linux_x86_64.tar.gz
RUN chmod +x ./grpcurl
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release -p thalo_scylla_event_indexer --bin thalo_scylla_event_indexer

# We do not need the Rust toolchain to run the binary!
FROM debian:bookworm-slim AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/thalo_scylla_event_indexer /usr/local/bin
COPY --from=builder /app/grpcurl /usr/local/bin/grpcurl
ENTRYPOINT ["/usr/local/bin/thalo_scylla_event_indexer"]
