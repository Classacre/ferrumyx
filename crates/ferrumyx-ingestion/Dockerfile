# Multi-stage Dockerfile for ferrumyx-ingestion
# Build stage
FROM rust:1.80-slim-bullseye AS builder

# Install CUDA toolkit for cross-compilation
RUN apt-get update && apt-get install -y --no-install-recommends \
    wget \
    && wget https://developer.download.nvidia.com/compute/cuda/repos/debian11/x86_64/cuda-keyring_1.1-1_all.deb \
    && dpkg -i cuda-keyring_1.1-1_all.deb \
    && apt-get update \
    && apt-get install -y --no-install-recommends \
    cuda-toolkit-12-2 \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Set CUDA environment variables for build
ENV CUDA_HOME=/usr/local/cuda
ENV PATH=$CUDA_HOME/bin:$PATH
ENV LD_LIBRARY_PATH=$CUDA_HOME/lib64:$LD_LIBRARY_PATH

WORKDIR /app

# Copy workspace files for dependency resolution
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build the release binary with CUDA support
RUN cargo build --release --package ferrumyx-ingestion --features cuda

# Runtime stage
FROM nvidia/cuda:12.2-runtime-ubuntu20.04

# Install minimal runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/* \
    && apt-get clean

# Set CUDA environment variables
ENV CUDA_HOME=/usr/local/cuda
ENV PATH=$CUDA_HOME/bin:$PATH
ENV LD_LIBRARY_PATH=$CUDA_HOME/lib64:$LD_LIBRARY_PATH

# Create non-root user
RUN useradd -r -s /bin/false ferrumyx

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/ferrumyx-ingestion /app/ferrumyx-ingestion

# Set proper ownership
RUN chown ferrumyx:ferrumyx /app/ferrumyx-ingestion

# Switch to non-root user
USER ferrumyx

# Expose port (assuming ingestion listens on 8082)
EXPOSE 8082

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=30s --retries=3 \
    CMD curl -f http://localhost:8082/health || exit 1

# Volumes for data and logs
VOLUME ["/app/data", "/app/logs"]

# Set environment variables (can be overridden)
ENV RUST_LOG=info \
    DATABASE_URL= \
    REDIS_URL= \
    PORT=8082 \
    INGESTION_CONFIG_PATH=/app/config/ingestion.toml

CMD ["./ferrumyx-ingestion"]