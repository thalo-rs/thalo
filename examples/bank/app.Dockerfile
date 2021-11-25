FROM rust

# Build dependencies only
WORKDIR /usr/src/app
RUN cargo new --lib thalo
COPY thalo/Cargo.toml thalo/
RUN cargo new --lib thalo-macros
COPY thalo-macros/Cargo.toml thalo-macros/
COPY Cargo.toml Cargo.lock ./
WORKDIR /usr/src/app/examples
RUN cargo new --bin bank
WORKDIR /usr/src/app/examples/bank
COPY examples/bank/Cargo.toml examples/bank/Cargo.lock ./
RUN cargo build --release

# Build workspace packages
WORKDIR /usr/src/app
COPY thalo/src thalo/src
RUN touch -a -m thalo/src/lib.rs
COPY thalo-macros/src thalo-macros/src
RUN touch -a -m thalo-macros/src/lib.rs
WORKDIR /usr/src/app/examples
RUN cargo build --release

# Build example package
WORKDIR /usr/src/app/examples/bank
COPY examples/bank/src src
RUN touch -a -m src/main.rs
RUN cargo install --path .
CMD ["bank"]
