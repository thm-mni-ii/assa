FROM rust:1.91.1-alpine AS build
RUN apk add --no-cache musl-dev
COPY ./common /common
COPY ./sql_feedback /app
WORKDIR /app
RUN --mount=type=cache,target=/usr/local/cargo/registry cargo build --release

FROM alpine:3
COPY --from=build /app/target/release/sql_feedback /usr/local/bin/sql_feedback
CMD ["/usr/local/bin/sql_feedback"]
