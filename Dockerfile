FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app/oauth-api

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/oauth-api/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json --bin oauth-api
# Build application
COPY . .
RUN cargo build --release --bin oauth-api

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/oauth-api/target/release/oauth-api /usr/local/bin
ENTRYPOINT /usr/local/bin/oauth-api
