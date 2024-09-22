FROM rustlang/rust:nightly AS builder
ENV DATABASE_URL="sqlite:db.sqlite"

WORKDIR /opt/moncorp-rustvoice
COPY . .
RUN cargo install sqlx-cli && cargo sqlx database setup && cargo install --path .

FROM debian:stable-slim
WORKDIR /opt/moncorp-rustvoice
COPY --from=builder /usr/local/cargo/bin/moncorp-rustvoice /usr/local/bin/moncorp-rustvoice
CMD ["moncorp-rustvoice"]

