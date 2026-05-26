FROM registry.hub.docker.com/library/rust:1.89-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY services/api services/api
RUN cargo build --release -p spillio-api

FROM registry.hub.docker.com/library/debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /app/target/release/spillio-api /usr/local/bin/spillio-api
COPY services/api/docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
RUN chmod +x /usr/local/bin/docker-entrypoint.sh
EXPOSE 4000
ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]
CMD ["spillio-api", "serve", "--addr", "0.0.0.0:4000"]
