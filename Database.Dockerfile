FROM rust:1.55-slim-buster
RUN apt-get update -y \
#    && apt-get install -y --no-install-recommends openssl \
    && apt-get install -y pkg-config \
    && apt-get install -y openssl \
    && apt-get install -y libssl-dev \
    # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*
RUN cargo install sqlx-cli --no-default-features --features postgres
ENV DB_USER=postgres
ENV DB_PASSWORD=password
ENV DB_NAME=propensity
ENV DB_HOST=propensity_postgres
ENV DB_PORT=5432
ENV DATABASE_URL=postgres://${DB_USER}:${DB_PASSWORD}@${DB_HOST}:${DB_PORT}/${DB_NAME}
COPY ./migrations ./migrations