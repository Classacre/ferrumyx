# Multi-stage Dockerfile for ferrumyx-agent
# Build stage
FROM rust:1.80-slim-bullseye AS builder

WORKDIR /app

# Copy workspace files for dependency resolution
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build the release binary
RUN cargo build --release --package ferrumyx-agent

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
COPY --from=builder /app/target/release/ferrumyx-agent /app/ferrumyx-agent

# Set proper ownership
RUN chown ferrumyx:ferrumyx /app/ferrumyx-agent

# Switch to non-root user
USER ferrumyx

# Expose port (assuming agent listens on 8081 or configurable)
EXPOSE 8081

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=30s --retries=3 \
    CMD curl -f http://localhost:8081/health || exit 1

# Volumes for data and logs
VOLUME ["/app/data", "/app/logs"]

# Set environment variables (can be overridden)
ENV RUST_LOG=info \
    DATABASE_URL= \
    REDIS_URL= \
    PORT=8081 \
    AGENT_CONFIG_PATH=/app/config/agent.toml

CMD ["./ferrumyx-agent"]