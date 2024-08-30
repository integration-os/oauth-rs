FROM lukemathwalker/cargo-chef:latest-rust-1.77.0 AS chef
WORKDIR /app/oauth-refresh

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/oauth-refresh/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json --bin oauth-refresh
# Build application
COPY . .
RUN cargo build --release --bin oauth-refresh

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/oauth-refresh/target/release/oauth-refresh /usr/local/bin
ENTRYPOINT /usr/local/bin/oauth-refresh
