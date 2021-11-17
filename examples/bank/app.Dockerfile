FROM rust

# Build dependencies only
WORKDIR /usr/src/app
RUN cargo new --lib awto-es
COPY awto-es/Cargo.toml awto-es/
RUN cargo new --lib awto-es-macros
COPY awto-es-macros/Cargo.toml awto-es-macros/
COPY Cargo.toml Cargo.lock ./
WORKDIR /usr/src/app/examples
RUN cargo new --bin bank
WORKDIR /usr/src/app/examples/bank
COPY examples/bank/Cargo.toml examples/bank/Cargo.lock ./
RUN cargo build --release

# Build workspace packages
WORKDIR /usr/src/app
COPY awto-es/src awto-es/src
RUN touch -a -m awto-es/src/lib.rs
COPY awto-es-macros/src awto-es-macros/src
RUN touch -a -m awto-es-macros/src/lib.rs
WORKDIR /usr/src/app/examples
RUN cargo build --release

# Build example package
WORKDIR /usr/src/app/examples/bank
COPY examples/bank/src src
RUN touch -a -m src/main.rs
RUN cargo install --path .
CMD ["bank"]
