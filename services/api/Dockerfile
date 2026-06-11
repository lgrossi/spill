FROM rust:1.89-bookworm AS build
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY services/api services/api
# `cli` is a workspace member in the root Cargo.toml; cargo needs every member's
# manifest present to load the workspace even though we only build spillio-api.
COPY cli cli
RUN cargo build --release -p spillio-api

FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=build /app/target/release/spillio-api /usr/local/bin/spillio-api
EXPOSE 8080
COPY services/api/docker-entrypoint.sh /usr/local/bin/docker-entrypoint.sh
RUN chmod +x /usr/local/bin/docker-entrypoint.sh
ENTRYPOINT ["docker-entrypoint.sh"]
CMD ["spillio-api", "serve"]
