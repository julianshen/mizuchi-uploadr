# syntax=docker/dockerfile:1

# Build stage
FROM --platform=$BUILDPLATFORM rust:1.85-bookworm AS builder

ARG TARGETPLATFORM
ARG BUILDPLATFORM

WORKDIR /app

# Install cross-compilation tools if needed
RUN case "$TARGETPLATFORM" in \
        "linux/arm64") \
            apt-get update && \
            apt-get install -y gcc-aarch64-linux-gnu && \
            rustup target add aarch64-unknown-linux-gnu \
            ;; \
    esac

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create dummy source and bench files to cache dependencies
RUN mkdir -p src benches && \
    echo "fn main() {}" > src/main.rs && \
    echo "pub fn lib() {}" > src/lib.rs && \
    echo "fn main() {}" > benches/upload_benchmark.rs && \
    echo "fn main() {}" > benches/zero_copy_benchmark.rs && \
    echo "fn main() {}" > benches/tracing_benchmark.rs

# Build dependencies (cache layer)
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release && \
    rm -rf src benches

# Copy actual source
COPY src ./src

# Build the application
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    touch src/main.rs src/lib.rs && \
    cargo build --release && \
    cp target/release/mizuchi-uploadr /usr/local/bin/

# Runtime stage
FROM debian:bookworm-slim AS runtime

# Labels for container registry
LABEL org.opencontainers.image.source="https://github.com/julianshen/mizuchi-uploadr"
LABEL org.opencontainers.image.description="High-performance upload-only S3 proxy"
LABEL org.opencontainers.image.licenses="MIT"

# Install runtime dependencies
# Note: libssl3 is named differently on some architectures (libssl3t64 on arm64)
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        curl && \
    # Install OpenSSL - try libssl3 first, fall back to libssl3t64
    (apt-get install -y --no-install-recommends libssl3 2>/dev/null || \
     apt-get install -y --no-install-recommends libssl3t64 2>/dev/null || \
     true) && \
    rm -rf /var/lib/apt/lists/* && \
    # Create non-root user (use UID 10001 to avoid conflicts with nobody/65534)
    useradd -r -s /bin/false -u 10001 mizuchi && \
    mkdir -p /etc/mizuchi /var/lib/mizuchi && \
    chown -R mizuchi:mizuchi /etc/mizuchi /var/lib/mizuchi

WORKDIR /app

# Copy binary from builder
COPY --from=builder /usr/local/bin/mizuchi-uploadr /usr/local/bin/

# Copy default config
COPY config.example.yaml /etc/mizuchi/config.yaml

USER mizuchi

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

EXPOSE 8080 9090

ENTRYPOINT ["mizuchi-uploadr"]
CMD ["--config", "/etc/mizuchi/config.yaml"]
