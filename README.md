# Mizuchi Uploadr

> *"水蛇 (Mizuchi) - The water dragon that secures your uploads"*

[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![CI](https://github.com/julianshen/mizuchi-uploadr/actions/workflows/ci.yml/badge.svg)](https://github.com/julianshen/mizuchi-uploadr/actions)

A high-performance **upload-only** S3 proxy built with Cloudflare's Pingora framework, featuring Linux kernel zero-copy optimization for maximum throughput.

## Features

- **Upload Only** - No download/list operations; security by design
- **S3 Compatible** - Works with AWS SDKs and S3 tools
- **Zero-Copy Transfer** - Linux `splice(2)`/`sendfile(2)` for 50-250x speedup
- **Cross-Platform** - Falls back to tokio buffered I/O on macOS/Windows
- **Flexible Auth** - JWT (HS256/RS256/ES256), AWS SigV4, JWKS endpoints
- **Fine-Grained AuthZ** - OPA policies, OpenFGA integration
- **Production Ready** - Prometheus metrics, OpenTelemetry tracing

## Performance

| File Size | Linux (zero-copy) | macOS/Windows (buffered) |
|-----------|-------------------|--------------------------|
| 1 MB | ~2 ms | ~10 ms |
| 10 MB | ~12 ms | ~600 ms |
| 50 MB | ~60 ms | ~15,000 ms |

## Quick Start

### Docker (Recommended)

```bash
# Pull and run
docker pull ghcr.io/julianshen/mizuchi-uploadr:latest
docker run -p 8080:8080 -v ./config.yaml:/etc/mizuchi/config.yaml \
  ghcr.io/julianshen/mizuchi-uploadr:latest

# Or use docker-compose with MinIO for local development
docker compose up -d

# Test upload
curl -X PUT http://localhost:8080/uploads/test.txt \
  -H "Content-Type: text/plain" \
  -d "Hello, Mizuchi!"
```

### From Source

```bash
git clone https://github.com/julianshen/mizuchi-uploadr.git
cd mizuchi-uploadr
cargo build --release
./target/release/mizuchi-uploadr --config config.example.yaml
```

## Configuration

```yaml
server:
  address: "0.0.0.0:8080"
  zero_copy:
    enabled: true
    pipe_buffer_size: 1048576  # 1MB

buckets:
  - name: "uploads"
    path_prefix: "/uploads"
    s3:
      bucket: "my-bucket"
      region: "us-east-1"
      access_key: "${AWS_ACCESS_KEY}"
      secret_key: "${AWS_SECRET_KEY}"
    auth:
      enabled: true
      jwt:
        secret: "${JWT_SECRET}"
        algorithm: "HS256"
    upload:
      multipart_threshold: 52428800  # 50MB
      part_size: 104857600           # 100MB
      concurrent_parts: 4

metrics:
  enabled: true
  port: 9090
```

## S3 API Compatibility

### Supported Operations

| Operation | Method | Endpoint |
|-----------|--------|----------|
| PutObject | PUT | `/{bucket}/{key}` |
| CreateMultipartUpload | POST | `/{bucket}/{key}?uploads` |
| UploadPart | PUT | `/{bucket}/{key}?partNumber=N&uploadId=X` |
| CompleteMultipartUpload | POST | `/{bucket}/{key}?uploadId=X` |
| AbortMultipartUpload | DELETE | `/{bucket}/{key}?uploadId=X` |
| ListParts | GET | `/{bucket}/{key}?uploadId=X` |

### NOT Supported (by design)

- GetObject, HeadObject (no downloads)
- ListObjects, ListBuckets (no listing)
- DeleteObject (no deletions)
- CopyObject (no server-side copy)

## Development

This project follows **strict TDD (Test-Driven Development)**. See [CLAUDE.md](CLAUDE.md) for development methodology.

```bash
# Run tests
cargo test

# Run with coverage
cargo tarpaulin --out Html

# Lint
cargo clippy --all-targets --all-features -- -D warnings

# Format
cargo fmt

# Benchmarks
cargo bench
```

## Project Structure

```
mizuchi-uploadr/
├── src/
│   ├── main.rs             # Entry point
│   ├── lib.rs              # Library root
│   ├── config/             # Configuration loading
│   ├── server/             # Pingora HTTP server
│   ├── router/             # S3 API path parsing
│   ├── auth/               # JWT, SigV4 authentication
│   ├── authz/              # OPA, OpenFGA authorization
│   ├── upload/             # Upload handlers, zero-copy
│   ├── s3/                 # S3 client, signing
│   └── metrics/            # Prometheus, tracing
├── tests/                  # Integration tests
├── benches/                # Performance benchmarks
└── docs/                   # Documentation
```

## Related Projects

- **[Yatagarasu](https://github.com/julianshen/yatagarasu)** - Read-only S3 proxy (sister project)
- **[Pingora](https://github.com/cloudflare/pingora)** - Underlying proxy framework

## License

MIT
