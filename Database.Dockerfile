FROM lukemathwalker/cargo-chef:latest-rust-1.55 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
ENV SQLX_OFFLINE true
RUN cargo build --release --bin loader

FROM rust:1.55
RUN apt-get update -y \
#    && apt-get install -y --no-install-recommends openssl \
    && apt-get install -y pkg-config \
    && apt-get install -y openssl \
    && apt-get install -y libssl-dev \
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/loader loader
COPY resources resources
RUN cargo install sqlx-cli --no-default-features --features postgres
ENV DB_USER=postgres
ENV DB_PASSWORD=password
ENV DB_NAME=propensity
ENV DB_PORT=5432
ENV APP__DATABASE__HOST=propensity_postgres
ENV DATABASE_URL=postgres://${DB_USER}:${DB_PASSWORD}@${APP__DATABASE__HOST}:${DB_PORT}/${DB_NAME}
COPY ./migrations ./migrations