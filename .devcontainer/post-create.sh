#!/bin/bash
set -e

echo "=== Mizuchi Uploadr Dev Container Setup ==="

# Ensure cargo registry is populated
echo "Fetching dependencies..."
cargo fetch

# Build the project to verify everything works
echo "Building project..."
cargo build

# Run clippy to verify code quality
echo "Running clippy..."
cargo clippy --all-targets --all-features -- -D warnings || true

# Display helpful information
echo ""
echo "=== Setup Complete ==="
echo ""
echo "Useful commands:"
echo "  cargo build              - Build the project"
echo "  cargo test               - Run all tests"
echo "  cargo test --lib         - Run unit tests only"
echo "  cargo clippy             - Run linter"
echo "  cargo fmt                - Format code"
echo "  cargo watch -x test      - Watch mode for tests"
echo "  cargo bench              - Run benchmarks"
echo ""
echo "Services (auto-started):"
echo "  MinIO Console: http://localhost:9001 (minioadmin/minioadmin)"
echo "  MinIO API:     http://localhost:9000"
echo ""
echo "Run the server:"
echo "  cargo run -- --config config.docker.yaml"
echo ""
