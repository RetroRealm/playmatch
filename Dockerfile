FROM lukemathwalker/cargo-chef:latest-rust-alpine AS chef
WORKDIR /playmatch

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
RUN apk add --no-cache curl
COPY --from=planner /playmatch/recipe.json recipe.json
# Dependency layer; cargo-chef caches it while sources change.
RUN cargo chef cook --release --locked --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --locked --bin playmatch

# The runtime image ships only the binary, no Rust toolchain.
FROM alpine:3.20 AS runtime
WORKDIR /playmatch
COPY --from=builder /playmatch/target/release/playmatch /usr/local/bin
RUN adduser -D -H playmatch && chown playmatch:playmatch /playmatch
USER playmatch
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/playmatch"]
