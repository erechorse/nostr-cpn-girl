FROM rust:1.71

RUN apt-get update && apt-get install -y \
    libpq-dev

RUN cargo install diesel_cli --no-default-features --features "postgres"
