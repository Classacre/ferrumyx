# Multi-stage Dockerfile for ferrumyx-ranker
# Build stage
FROM rust:1.80-slim-bullseye AS builder

WORKDIR /app

# Copy workspace files for dependency resolution
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build the release binary
RUN cargo build --release --package ferrumyx-ranker

# Runtime stage
FROM debian:bullseye-slim

# Install minimal runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Create non-root user
RUN useradd -r -s /bin/false ferrumyx

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/ferrumyx-ranker /app/ferrumyx-ranker

# Set proper ownership
RUN chown ferrumyx:ferrumyx /app/ferrumyx-ranker

# Switch to non-root user
USER ferrumyx

# Expose port (assuming ranker listens on 8084)
EXPOSE 8084

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=30s --retries=3 \
    CMD curl -f http://localhost:8084/health || exit 1

# Volumes for data and logs
VOLUME ["/app/data", "/app/logs"]

# Set environment variables (can be overridden)
ENV RUST_LOG=info \
    DATABASE_URL= \
    REDIS_URL= \
    PORT=8084 \
    RANKER_CONFIG_PATH=/app/config/ranker.toml

CMD ["./ferrumyx-ranker"]