# syntax=docker/dockerfile:1

# Build stage
FROM --platform=$BUILDPLATFORM rust:1.83-bookworm AS builder

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

# Create dummy source to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    echo "pub fn lib() {}" > src/lib.rs

# Build dependencies (cache layer)
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release && \
    rm -rf src

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

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 \
        curl && \
    rm -rf /var/lib/apt/lists/* && \
    # Create non-root user
    useradd -r -s /bin/false -u 65534 mizuchi && \
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
