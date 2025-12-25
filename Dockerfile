# Build stage
FROM rust:1.75-bookworm as builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock* ./

# Create dummy source to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    echo "pub fn lib() {}" > src/lib.rs

# Build dependencies
RUN cargo build --release && \
    rm -rf src

# Copy actual source
COPY src ./src

# Build the application
RUN touch src/main.rs src/lib.rs && \
    cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && \
    apt-get install -y --no-install-recommends \
        ca-certificates \
        libssl3 && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /app/target/release/mizuchi-uploadr /usr/local/bin/

# Copy config
COPY config.example.yaml /etc/mizuchi/config.yaml

# Create non-root user
RUN useradd -r -s /bin/false mizuchi && \
    chown -R mizuchi:mizuchi /etc/mizuchi

USER mizuchi

EXPOSE 8080 9090

ENTRYPOINT ["mizuchi-uploadr"]
CMD ["--config", "/etc/mizuchi/config.yaml"]
