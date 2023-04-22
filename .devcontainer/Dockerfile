FROM rust:1.68.2

RUN apt-get update && apt-get install -y \
    libpq-dev

RUN cargo install diesel_cli --no-default-features --features "postgres"
