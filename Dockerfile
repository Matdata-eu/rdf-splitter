# ── Build stage ────────────────────────────────────────────────────────────────
FROM rust:1.93.1-slim AS builder

WORKDIR /build

# Cache dependencies before copying source
COPY Cargo.toml Cargo.lock* ./
RUN mkdir src && echo 'fn main(){}' > src/main.rs \
 && cargo build --release \
 && rm -rf src

# Build real source
COPY src ./src
RUN touch src/main.rs \
 && cargo build --release

# ── Runtime stage ──────────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates \
 && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/rdfsplitter /usr/local/bin/rdfsplitter

WORKDIR /data

ENTRYPOINT ["rdfsplitter"]
CMD ["--help"]
