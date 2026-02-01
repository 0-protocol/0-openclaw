# 0-openclaw Dockerfile
# Multi-stage build for smaller final image

FROM rust:1.75-slim as builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy Cargo files first for dependency caching
COPY Cargo.toml Cargo.lock* ./

# Create dummy src to build dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    echo "pub fn dummy() {}" > src/lib.rs && \
    cargo build --release && \
    rm -rf src

# Copy actual source
COPY . .

# Build release binary
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/zero-openclaw /usr/local/bin/

# Copy graphs
COPY --from=builder /app/graphs /app/graphs

# Create data directory
RUN mkdir -p /data /app/config

# Default port
EXPOSE 18789

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD zero-openclaw status || exit 1

ENTRYPOINT ["zero-openclaw"]
CMD ["gateway", "--port", "18789"]
