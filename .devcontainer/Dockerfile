# Dev Container for Mizuchi Uploadr
# Based on Rust official image with additional dev tools

FROM mcr.microsoft.com/devcontainers/rust:1-bookworm

# Install additional system dependencies
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get -y install --no-install-recommends \
        # Build tools
        build-essential \
        pkg-config \
        # SSL/TLS
        libssl-dev \
        ca-certificates \
        # Debugging/profiling
        linux-perf \
        strace \
        # Python for E2E tests
        python3 \
        python3-pip \
        python3-venv \
        # Utilities
        curl \
        jq \
        httpie \
        # Clean up
    && apt-get autoremove -y \
    && apt-get clean -y \
    && rm -rf /var/lib/apt/lists/*

# Install Rust toolchain components
RUN rustup component add \
    clippy \
    rustfmt \
    rust-src \
    rust-analyzer

# Install cargo tools for development
RUN cargo install --locked \
    cargo-watch \
    cargo-audit \
    cargo-outdated \
    cargo-criterion \
    && rm -rf /usr/local/cargo/registry/cache

# Set up Python virtual environment for E2E tests
ENV VIRTUAL_ENV=/opt/venv
RUN python3 -m venv $VIRTUAL_ENV
ENV PATH="$VIRTUAL_ENV/bin:$PATH"

# Install Python dependencies for E2E tests
RUN pip install --no-cache-dir \
    boto3 \
    requests \
    pyjwt \
    pytest \
    pytest-asyncio

# Set working directory
WORKDIR /workspaces/mizuchi-uploadr

# Configure git for safe directory (codespaces requirement)
RUN git config --global --add safe.directory /workspaces/mizuchi-uploadr
