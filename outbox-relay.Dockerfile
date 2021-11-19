FROM rust:1.56 as builder

# create new empty shell project
RUN USER=root cargo new --bin outbox-relay
WORKDIR /outbox-relay

# copy manifests
COPY ./Cargo.lock ./Cargo.lock
COPY ./outbox-relay/Cargo.toml ./Cargo.toml

# build only dependencies
RUN cargo build --release
RUN rm src/*.rs

# copy source code
COPY ./outbox-relay/src ./src

# build
RUN rm ./target/release/deps/outbox_relay*
COPY ./outbox-relay/README.md ./README.md
RUN cargo build --release

# copy 
FROM rust:1.5
COPY --from=builder /outbox-relay/target/release/outbox-relay .

# install postgres client
RUN apt-get update
RUN apt-get install -y postgresql-client
RUN rm -rf /var/lib/apt/lists/*

# run from env vars
CMD ./outbox-relay --database-url=$DATABASE_URL --redpanda-host=$REDPANDA_HOST --slot=$SLOT
