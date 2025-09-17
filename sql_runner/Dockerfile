FROM rust:1.85.1-alpine AS build
RUN apk add --no-cache musl-dev
COPY . /app
WORKDIR /app
RUN --mount=type=cache,target=/usr/local/cargo/registry cargo build --release

FROM alpine:3
COPY --from=build /app/target/release/sql_runner /usr/local/bin/sql_runner
CMD ["/usr/local/bin/sql_runner"]
