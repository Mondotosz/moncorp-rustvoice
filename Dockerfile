FROM rust:alpine AS builder

RUN apk add --no-cache \
    musl-dev \
    sqlite-dev \
    pkgconf

WORKDIR /build

# Copy manifests first for layer caching
COPY Cargo.toml Cargo.lock ./
COPY crates/rustvoice/Cargo.toml crates/rustvoice/
COPY crates/bot/Cargo.toml      crates/bot/
COPY crates/db/Cargo.toml       crates/db/
COPY crates/ipc/Cargo.toml      crates/ipc/

# Stub out each crate so Cargo can fetch and build dependencies before sources change
RUN for crate in rustvoice bot db ipc; do \
      mkdir -p crates/$crate/src && echo "fn main(){}" > crates/$crate/src/main.rs && touch crates/$crate/src/lib.rs; \
    done

RUN cargo build --release -p rustvoice 2>/dev/null; true

# Now replace stubs with real sources and rebuild only what changed
COPY crates crates
RUN find crates -name "*.rs" | xargs touch

RUN cargo build --release -p rustvoice


FROM alpine

RUN apk add --no-cache \
    sqlite-libs \
    ca-certificates

COPY --from=builder /build/target/release/rustvoice /usr/local/bin/rustvoice

ENTRYPOINT ["rustvoice"]
CMD ["run"]
