# Apophy Sovereign - Multi-stage Docker build
# Single binary, minimal attack surface

# Stage 1: Build
FROM rust:1.82-slim-bookworm AS builder

WORKDIR /build

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace
COPY Cargo.toml Cargo.lock* ./
COPY crates/ crates/

# Build release binary
RUN cargo build --release --bin apophy-sovereign

# Stage 2: Runtime (minimal)
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd -r -s /bin/false apophy

WORKDIR /app

# Copy binary
COPY --from=builder /build/target/release/apophy-sovereign /app/apophy-sovereign

# Copy default config
COPY config/ /app/config/

# Create data directories
RUN mkdir -p /app/data /app/models /app/keys \
    && chown -R apophy:apophy /app

USER apophy

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

ENTRYPOINT ["/app/apophy-sovereign"]
CMD ["start", "--config", "/app/config/sovereign.toml"]
