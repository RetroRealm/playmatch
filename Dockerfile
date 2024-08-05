# ---------------------------------------------------
# 1 - Build Stage
#
# Use official rust image to for application docker-build.yml
# ---------------------------------------------------
FROM rust:1.80 AS build

# Setup working directory
WORKDIR /usr/src/playmatch
COPY . .

# Build application
RUN cargo install --path .

# ---------------------------------------------------
# 2 - Deploy Stage
#
# Use a distroless image for minimal container size
# ---------------------------------------------------
FROM gcr.io/distroless/cc-debian12

# Application files
COPY --from=build /usr/local/cargo/bin/playmatch /usr/local/bin/playmatch

CMD ["playmatch"]
