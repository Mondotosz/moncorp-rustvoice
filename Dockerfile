FROM lukemathwalker/cargo-chef:latest-rust-alpine AS chef
RUN apk add --no-cache musl-dev sqlite-dev pkgconf
WORKDIR /build

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /build/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --locked -p rustvoice

FROM alpine:3.21 AS runtime
RUN apk add --no-cache sqlite-libs ca-certificates
COPY --from=builder /build/target/release/rustvoice /usr/local/bin/rustvoice
ENTRYPOINT ["rustvoice"]
CMD ["run"]
