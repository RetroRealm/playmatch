FROM clux/muslrust:stable AS chef
USER root
RUN cargo install cargo-chef
WORKDIR /playmatch

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
RUN apt install -y musl-tools curl
COPY --from=planner /playmatch/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --target x86_64-unknown-linux-musl --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --target x86_64-unknown-linux-musl --bin playmatch

# We do not need the Rust toolchain to run the binary!
FROM alpine:3.20 AS runtime
WORKDIR /playmatch
COPY --from=builder /playmatch/target/x86_64-unknown-linux-musl/release/playmatch /usr/local/bin
ENTRYPOINT ["/usr/local/bin/playmatch"]
