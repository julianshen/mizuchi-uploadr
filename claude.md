# Claude Context: Mizuchi Uploadr

> This file provides context for AI assistants working on the Mizuchi Uploadr codebase.

## Project Overview

**Mizuchi Uploadr** (水蛇 - "water dragon") is a high-performance, upload-only S3 proxy built in Rust. It provides an S3-compatible REST API with Linux kernel zero-copy optimization.

### Key Design Decisions

1. **Upload-only proxy** - No download/list operations; security by design
2. **S3-compatible API** - Works with AWS SDKs and S3 tools
3. **Linux zero-copy** - Uses `splice(2)`/`sendfile(2)` for kernel-space transfers
4. **Cross-platform** - Falls back to tokio buffered I/O on macOS/Windows
5. **50MB multipart threshold** - Automatic chunking for large files

## Development Methodology

### TDD: Red-Green-Refactor Cycle

This project follows **strict Test-Driven Development (TDD)**. All new features and bug fixes MUST follow the Red-Green-Refactor cycle:

```
┌─────────────────────────────────────────────────────────────┐
│                  TDD Development Cycle                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│   🔴 RED ──────► 🟢 GREEN ──────► 🔵 REFACTOR ──────┐       │
│    │                                                 │       │
│    │  Write failing    Minimal code     Clean up    │       │
│    │  test first       to pass test     the code    │       │
│    │                                                 │       │
│    └─────────────────────────────────────────────────┘       │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

#### Phase 1: RED - Write Failing Test 🔴
- Write a test that defines expected behavior
- Run the test - it MUST fail
- Ensure test fails for the RIGHT reason (not syntax errors)
- Test should be minimal and focused

#### Phase 2: GREEN - Make It Pass 🟢
- Write the MINIMUM code to pass the test
- Do not add extra features or optimizations
- Focus only on making the test green
- Resist the urge to write "complete" code

#### Phase 3: REFACTOR - Clean Up 🔵
- Improve code quality while keeping tests green
- Remove duplication
- Improve naming and structure
- Run tests after EVERY change

### Development Workflow

```bash
# 1. Create feature branch
git checkout -b feature/new-capability

# 2. RED: Write failing test
cargo test --lib test_new_feature  # Should FAIL

# 3. GREEN: Implement minimal code
cargo test --lib test_new_feature  # Should PASS

# 4. REFACTOR: Clean up
cargo test  # All tests should PASS

# 5. Run full test suite before commit
cargo test --all-features
cargo clippy -- -D warnings
cargo fmt --check

# 6. Commit with meaningful message
git commit -m "feat(upload): add new capability

- RED: Added test for X behavior
- GREEN: Implemented minimal X handler
- REFACTOR: Extracted common logic to trait"
```

### Test Categories

| Category | Location | Command | Purpose |
|----------|----------|---------|---------|
| Unit Tests | `src/**/*.rs` (inline) | `cargo test --lib` | Test individual functions |
| Integration Tests | `tests/` | `cargo test --test '*'` | Test component interactions |
| Benchmarks | `benches/` | `cargo bench` | Performance regression |
| E2E Tests | `test_*.py` | `python3 test_zero_copy.py` | Full system validation |

### Quality Gates

Before any code is merged, it MUST pass:

1. **All Tests Pass**: `cargo test --all-features`
2. **No Clippy Warnings**: `cargo clippy -- -D warnings`
3. **Formatted Code**: `cargo fmt --check`
4. **Documentation**: `cargo doc --no-deps`
5. **No Security Issues**: `cargo audit` (if available)

### Commit Discipline

Follow **RGRC** (Red-Green-Refactor-Commit):

```
feat(module): short description

- RED: What test was written first
- GREEN: What minimal implementation was added
- REFACTOR: What improvements were made

Closes #123
```

### Pull Request Rule

**MANDATORY**: Create a Pull Request for review after completing each development phase.

```
┌─────────────────────────────────────────────────────────────┐
│              PR-per-Phase Development Flow                   │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│   🔴 RED ──► PR #1 ──► Review ──► Merge                     │
│                                    │                         │
│   🟢 GREEN ──► PR #2 ──► Review ──► Merge                   │
│                                    │                         │
│   🔵 REFACTOR ──► PR #3 ──► Review ──► Merge                │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

**Why PR per phase?**
- Enables incremental code review
- Catches issues early before they compound
- Documents the TDD journey in git history
- Allows team to verify RED phase actually fails
- Keeps PRs small and reviewable

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Mizuchi Uploadr                          │
├─────────────────────────────────────────────────────────────┤
│  HTTP Layer (Pingora)                                        │
│  ├── S3 API Router (PUT, POST, DELETE)                      │
│  ├── Auth (JWT/SigV4)                                        │
│  └── AuthZ (OPA/OpenFGA)                                     │
├─────────────────────────────────────────────────────────────┤
│  Upload Layer                                                │
│  ├── Simple Upload (≤50MB)                                   │
│  ├── Multipart Upload (>50MB)                                │
│  └── Zero-Copy Transfer                                      │
│      ├── Linux: splice(2)/sendfile(2)                        │
│      └── Other: tokio buffered I/O                           │
├─────────────────────────────────────────────────────────────┤
│  S3 Client Layer                                             │
│  ├── Connection Pool                                         │
│  ├── SigV4 Signing                                           │
│  └── Multipart Orchestration                                 │
└─────────────────────────────────────────────────────────────┘
```

## Directory Structure

```
mizuchi-uploadr/
├── Cargo.toml              # Dependencies with platform-specific sections
├── src/
│   ├── main.rs             # Entry point, CLI, server startup
│   ├── lib.rs              # Library root, public API
│   ├── config/             # YAML config loading
│   ├── server/             # Pingora HTTP server
│   ├── router/             # S3 API path parsing
│   ├── auth/               # JWT, SigV4 validation
│   ├── authz/              # OPA, OpenFGA integration
│   ├── upload/
│   │   ├── mod.rs          # Upload orchestration
│   │   ├── zero_copy.rs    # Platform-specific transfer (IMPORTANT)
│   │   ├── put_object.rs   # Simple upload handler
│   │   └── multipart.rs    # Multipart upload handler
│   ├── s3/                 # S3 client, signing
│   └── metrics/            # Prometheus, tracing
├── benches/                # Criterion benchmarks
└── tests/                  # Integration tests
```

## Critical Files

### `src/upload/zero_copy.rs`
This is the core innovation of the project. It contains:

1. **Linux implementation** (`#[cfg(target_os = "linux")]`)
   - `ZeroCopyTransfer` struct with pipe file descriptors
   - `splice()` for socket→pipe→socket transfers
   - `sendfile()` for file→socket transfers
   - Uses `nix` crate for syscall bindings

2. **Fallback implementation** (`#[cfg(not(target_os = "linux"))]`)
   - Uses tokio's `AsyncRead`/`AsyncWrite`
   - Buffered transfer with configurable chunk size

3. **Platform-agnostic API** (`DataTransfer`)
   - Single interface for both implementations
   - Runtime detection of capabilities

### Platform Detection Pattern
```rust
#[cfg(target_os = "linux")]
mod linux { /* splice/sendfile */ }

#[cfg(not(target_os = "linux"))]
mod fallback { /* tokio buffered */ }

pub struct DataTransfer {
    #[cfg(target_os = "linux")]
    inner: Option<linux::ZeroCopyTransfer>,
    #[cfg(not(target_os = "linux"))]
    inner: fallback::ZeroCopyTransfer,
}
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
    upload:
      multipart_threshold: 52428800  # 50MB
      part_size: 104857600           # 100MB
      concurrent_parts: 4
```

## Development Tasks (TDD Approach)

### "Add a new S3 operation"
1. 🔴 **RED**: Write failing integration test for the new operation
2. 🔴 **RED**: Write unit test for parsing in `S3RequestParser`
3. 🟢 **GREEN**: Add variant to `S3Operation` enum in `router/s3_parser.rs`
4. 🟢 **GREEN**: Add parsing logic in `S3RequestParser::parse()`
5. 🟢 **GREEN**: Add handler in `upload/handler.rs`
6. 🔵 **REFACTOR**: Extract common patterns, improve naming
7. ✅ **VERIFY**: Run full test suite

### "Support a new auth method"
1. 🔴 **RED**: Write failing test for auth validation
2. 🔴 **RED**: Write test for config loading
3. 🟢 **GREEN**: Add to `auth/` module
4. 🟢 **GREEN**: Implement `Authenticator` trait
5. 🟢 **GREEN**: Add config option
6. 🟢 **GREEN**: Add to auth chain in `server/service.rs`
7. 🔵 **REFACTOR**: Ensure consistent error handling
8. ✅ **VERIFY**: Run full test suite

### "Optimize transfer performance"
1. 🔴 **RED**: Write benchmark capturing current performance
2. Check `zero_copy.rs` first
3. 🟢 **GREEN**: Tune `pipe_buffer_size` in config
4. 🟢 **GREEN**: Consider `SPLICE_F_MORE` flag usage
5. 🔵 **REFACTOR**: Profile with `perf` on Linux
6. ✅ **VERIFY**: Compare benchmarks, ensure no regressions

### "Fix a bug"
1. 🔴 **RED**: Write a test that reproduces the bug (test MUST fail)
2. 🟢 **GREEN**: Fix the bug with minimal code change
3. 🔵 **REFACTOR**: Check for similar issues elsewhere
4. ✅ **VERIFY**: Ensure fix doesn't break other tests

## Common Patterns

**Error Handling**
```rust
use thiserror::Error;

#[derive(Error, Debug)]
pub enum UploadError {
    #[error("S3 error: {0}")]
    S3Error(#[from] aws_sdk_s3::Error),
    
    #[error("Zero-copy not available")]
    ZeroCopyUnavailable,
}
```

**Metrics**
```rust
use prometheus::{Counter, Histogram};

lazy_static! {
    static ref ZERO_COPY_BYTES: Counter = register_counter!(
        "zero_copy_bytes_total",
        "Bytes transferred via splice/sendfile"
    ).unwrap();
}
```

**Async with splice**
```rust
// splice() is blocking, so we yield to tokio runtime
match nix::fcntl::splice(...) {
    Err(Errno::EAGAIN) => {
        tokio::task::yield_now().await;
        continue;
    }
    // ...
}
```

## Performance Expectations

| Size | Linux (zero-copy) | macOS/Windows (buffered) |
|------|-------------------|--------------------------|
| 1 MB | ~2 ms | ~10 ms |
| 10 MB | ~12 ms | ~600 ms |
| 50 MB | ~60 ms | ~15,000 ms |

Zero-copy provides **50-250x speedup** for large files on Linux.

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `tokio` | Async runtime |
| `hyper` | HTTP server |
| `aws-sdk-s3` | S3 client |
| `nix` | Linux syscalls (Linux only) |
| `jsonwebtoken` | JWT validation |
| `prometheus` | Metrics |

## Gotchas

1. **splice() requires Linux 2.6.17+** - Always provide fallback
2. **Pipe buffer size** - Default 64KB, can increase with `F_SETPIPE_SZ`
3. **SPLICE_F_MOVE** - Hint only, kernel may still copy
4. **Non-blocking sockets** - Must handle `EAGAIN` and yield
5. **Multipart minimum** - S3 requires 5MB minimum per part (except last)

## TDD Anti-Patterns to Avoid

❌ **Don't** write implementation before tests
❌ **Don't** skip the RED phase (test must fail first)
❌ **Don't** write more code than needed to pass the test
❌ **Don't** refactor while tests are failing
❌ **Don't** merge without running full test suite
❌ **Don't** ignore flaky tests (fix them immediately)

## References

### Yatagarasu (Sister Project)

**[Yatagarasu](https://github.com/julianshen/yatagarasu)** - Read-only S3 proxy built with Pingora framework.

Mizuchi Uploadr can **reuse implementations** from Yatagarasu:
- **JWT Authentication** (`src/auth/`) - HS256/RS256/ES256, JWKS endpoints
- **OPA Authorization** (`src/authz/opa/`) - Policy-based access control
- **OpenFGA Authorization** (`src/authz/openfga/`) - Fine-grained authorization
- **Config Loading** (`src/config/`) - YAML configuration patterns
- **Metrics** (`src/metrics/`) - Prometheus, OpenTelemetry tracing

### External References

- [Linux splice(2) man page](https://man7.org/linux/man-pages/man2/splice.2.html)
- [Linux sendfile(2) man page](https://man7.org/linux/man-pages/man2/sendfile.2.html)
- [AWS S3 REST API Reference](https://docs.aws.amazon.com/AmazonS3/latest/API/)
- [Pingora Framework](https://github.com/cloudflare/pingora)
- [nix crate docs](https://docs.rs/nix/)
- [TDD Red-Green-Refactor](https://www.codecademy.com/article/tdd-red-green-refactor)
