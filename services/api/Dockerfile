FROM rust:1.89-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY services/api services/api
RUN cargo build --release -p spillio-api

FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /app/target/release/spillio-api /usr/local/bin/spillio-api
EXPOSE 4000
CMD ["spillio-api", "serve", "--addr", "0.0.0.0:4000"]
