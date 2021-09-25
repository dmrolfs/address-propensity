FROM lukemathwalker/cargo-chef:latest-rust-1.55-slim-buster AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
# Compute a lock-like file for our project
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build our project dependencies, not our application!
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
ENV SQLX_OFFLINE true
# Build our project
RUN cargo build --release

FROM debian:buster-slim AS runtime
WORKDIR /app
RUN apt-get update -y \
    && apt-get install -y --no-install-recommends openssl \
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/server server
COPY --from=builder /app/target/release/loader loader
COPY resources resources
ENV APP_ENVIRONMENT production
ENV APP__DATABASE__HOST propensity_postgres
ENV RUST_LOG info
#ENTRYPOINT ["./server", "-s", "resources/secrets.yaml"]
