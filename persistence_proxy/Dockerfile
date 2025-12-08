FROM rust:1.85.1-alpine AS build
RUN apk add --no-cache musl-dev
COPY ./common /common
COPY ./persistence_proxy /app
WORKDIR /app
RUN --mount=type=cache,target=/usr/local/cargo/registry cargo build --release && cd migration && cargo build --release

FROM alpine:3
COPY --from=build /app/target/release/persistence_proxy /usr/local/bin/persistence_proxy
COPY --from=build /app/migration/target/release/migration /usr/local/bin/persistence_proxy_migration
CMD ["/usr/local/bin/persistence_proxy"]
