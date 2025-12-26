# Mizuchi Uploadr - Master Implementation Plan

> **Project**: High-performance upload-only S3 proxy with zero-copy optimization
> **Methodology**: Strict TDD (Test-Driven Development) - Red-Green-Refactor Cycle
> **Last Updated**: 2025-12-25
> **Status**: Active Development - Tracing Phase 4 In Progress

---

## 📊 Overall Progress

### Project Status

| Component               | Status         | Progress | PRs   | Tests      |
| ----------------------- | -------------- | -------- | ----- | ---------- |
| **Tracing**             | 🚧 In Progress | 85%      | 19/22 | ✅ Passing |
| **Core Infrastructure** | ⏳ Not Started | 0%       | 0/9   | -          |
| **Upload Operations**   | ⏳ Not Started | 0%       | 0/9   | -          |
| **Authentication**      | ⏳ Not Started | 0%       | 0/9   | -          |
| **Authorization**       | ⏳ Not Started | 0%       | 0/6   | -          |
| **Production Ready**    | ⏳ Not Started | 0%       | 0/6   | -          |

### Tracing Progress Detail

| Phase                | Status         | PRs    | Tests         | Completion |
| -------------------- | -------------- | ------ | ------------- | ---------- |
| 1. Configuration     | ✅ Complete    | #4     | 8/8 passing   | 100%       |
| 2. OTel Integration  | ✅ Complete    | #5-16  | All passing   | 100%       |
| 3. Instrumentation   | ✅ Complete    | #13    | All passing   | 100%       |
| 4. Advanced Features | 🚧 In Progress | #17-19 | 14/14 passing | 75%        |
| 5. Production Ready  | ⏳ Not Started | -      | 0/8           | 0%         |

---

## 🎯 Priority Queue (Next Tasks)

### 🔴 Immediate Priorities (This Week)

#### 1. ✅ Complete Tracing Phase 4.2 - W3C Trace Context Extraction

- **Status**: ✅ COMPLETE (2025-12-25)
- **Goal**: Extract traceparent from incoming HTTP requests
- **Completed**: Span linking with W3C Trace Context
- **Impact**: Enables end-to-end distributed tracing
- **Files**: `src/tracing/instrumentation.rs`, `tests/context_propagation_test.rs`
- **Commit**: 4d30ca3

#### 2. ✅ Complete Tracing Phase 4.3 - Auth/AuthZ Tracing

- **Status**: ✅ COMPLETE (2025-12-25)
- **Goal**: Add tracing to authentication and authorization operations
- **Completed**: Instrumentation added to all auth/authz operations
- **Impact**: Complete observability for security operations with no PII leakage
- **Files**: `src/auth/jwt.rs`, `src/auth/sigv4.rs`, `src/authz/opa/mod.rs`, `src/authz/openfga/mod.rs`
- **Commit**: 301dd24

#### 3. Complete Tracing Phase 5 - Production Readiness

- **Goal**: Error handling, resilience, and comprehensive documentation
- **Estimated**: 3-4 days
- **Depends On**: Task 2
- **Impact**: Production-ready tracing implementation
- **Files**: `src/tracing/error.rs`, `docs/TRACING.md`, `examples/tracing_*.rs`

### 🟡 Short-term Priorities (Next 2 Weeks)

#### 4. Phase 1.1 - HTTP Server with Pingora Framework

- **Goal**: Replace placeholder server with actual Pingora-based HTTP server
- **Estimated**: 1 week
- **Blocker**: Tracing must be complete
- **Impact**: Core HTTP functionality
- **Files**: `src/server/pingora_service.rs`, `tests/server_test.rs`

#### 5. Phase 1.2 - Request Router Enhancement

- **Goal**: Complete S3 request routing with bucket resolution
- **Estimated**: 3-4 days
- **Depends On**: Task 4
- **Impact**: Multi-bucket support
- **Files**: `src/router/bucket_resolver.rs`, `tests/router_test.rs`

#### 6. Phase 1.3 - S3 Client Integration

- **Goal**: Integrate AWS SDK for actual S3 operations
- **Estimated**: 1 week
- **Depends On**: Task 5
- **Impact**: Real S3 connectivity
- **Files**: `src/s3/client.rs`, `src/s3/credentials.rs`, `tests/s3_client_test.rs`

### 🟢 Medium-term Priorities (Next Month)

#### 7. Phase 2.1 - Simple PutObject Handler

- **Goal**: Implement single-part upload for files ≤50MB
- **Estimated**: 1 week
- **Depends On**: Task 6
- **Impact**: Basic upload functionality

#### 8. Phase 2.2 - Multipart Upload Handler

- **Goal**: Implement multipart upload for files >50MB
- **Estimated**: 1 week
- **Depends On**: Task 7
- **Impact**: Large file support

#### 9. Phase 2.3 - Zero-Copy Integration

- **Goal**: Integrate zero-copy transfer into upload handlers
- **Estimated**: 1 week
- **Depends On**: Task 8
- **Impact**: 50-250x performance improvement on Linux

---

## 📋 Recent Completions

### ✅ PR #17 (2025-12-25): W3C Trace Context Injection for S3

- Added `inject_trace_context()` to S3Client
- All S3 operations now inject traceparent header
- 5 new integration tests passing
- Complete TDD cycle (RED-GREEN-REFACTOR)

### ✅ PR #13: HTTP Span Instrumentation

- HTTP requests create spans with semantic conventions
- HTTP attributes (method, path, status_code, content_length)

### ✅ PR #11: Advanced Sampling

- Configurable sampling strategies (always, never, ratio, parent_based)

### ✅ PR #12: Performance Optimization

- Optimized trace context propagation
- Reduced tracing overhead

---

## 🚧 IN PROGRESS: Tracing Phase 4 - Advanced Features

### Phase 4.2: W3C Trace Context Extraction ⏳ NOT STARTED

**Goal**: Extract trace context from incoming HTTP requests

**TDD Workflow**:

1. 🔴 **RED**: Write failing tests

   - Test: Extract traceparent header from HTTP request
   - Test: Parse W3C Trace Context format
   - Test: Create span context from extracted headers
   - Test: Invalid traceparent handling

2. 🟢 **GREEN**: Implement extraction

   - Create `src/tracing/propagation.rs`
   - Extract `traceparent` and `tracestate` headers
   - Parse W3C format (version-trace_id-span_id-flags)
   - Link spans correctly

3. 🔵 **REFACTOR**: Optimize
   - Reduce parsing overhead
   - Add validation for trace context
   - Improve error handling

**Files to Create/Modify**:

- `src/tracing/propagation.rs` (new)
- `src/server/tracing_middleware.rs` (modify)
- `tests/context_propagation_test.rs` (modify)

**Acceptance Criteria**:

- [ ] Trace context extracted from incoming requests
- [ ] Distributed traces work end-to-end
- [ ] Invalid trace context handled gracefully
- [ ] All tests pass

---

### Phase 4.3: Auth/AuthZ Tracing ⏳ NOT STARTED

**Goal**: Add tracing to authentication and authorization operations

**TDD Workflow**:

1. 🔴 **RED**: Write failing tests

   - Test: JWT validation creates span
   - Test: SigV4 validation creates span
   - Test: OPA/OpenFGA calls traced
   - Test: No PII in span attributes

2. 🟢 **GREEN**: Implement auth tracing

   - Add `#[instrument]` to JWT validators
   - Add `#[instrument]` to SigV4 validators
   - Add spans to OPA client
   - Add spans to OpenFGA client

3. 🔵 **REFACTOR**: Add security attributes
   - Add user ID (hashed, no PII)
   - Add auth method used
   - Add authorization decision (allow/deny)

**Files to Create/Modify**:

- `src/auth/jwt.rs` (modify)
- `src/auth/sigv4.rs` (modify)
- `src/authz/opa/mod.rs` (modify)
- `src/authz/openfga/mod.rs` (modify)
- `tests/auth_tracing_test.rs` (new)

**Acceptance Criteria**:

- [ ] Auth operations traced (no PII)
- [ ] Authorization decisions logged
- [ ] All tests pass

---

### Phase 4.4: Performance Optimization ⏳ NOT STARTED

**Goal**: Optimize tracing performance and measure overhead

**TDD Workflow**:

1. 🔴 **RED**: Write benchmark tests

   - Benchmark: Request with tracing vs without
   - Benchmark: Span creation overhead
   - Benchmark: OTLP export latency

2. 🟢 **GREEN**: Implement optimizations

   - Reduce tracing overhead
   - Optimize span creation
   - Tune batch export settings

3. 🔵 **REFACTOR**: Performance tuning
   - Profile and optimize hot paths
   - Reduce allocations

**Files to Create/Modify**:

- `benches/tracing_benchmark.rs` (new)
- `src/tracing/init.rs` (modify)

**Acceptance Criteria**:

- [ ] Sampling strategies configurable
- [ ] Performance overhead < 5%
- [ ] Benchmarks show acceptable overhead

---

## ⏳ PLANNED: Tracing Phase 5 - Production Readiness

### Phase 5.1: Error Handling & Resilience

**Goal**: Ensure application continues if tracing fails

**TDD Workflow**:

1. 🔴 **RED**: Write tests for failure scenarios

   - Test: OTLP backend unavailable
   - Test: Network timeout
   - Test: Application continues on tracing failure

2. 🟢 **GREEN**: Implement error handling

   - Create `src/tracing/error.rs`
   - Add retry logic for OTLP export
   - Fallback to console logging
   - Circuit breaker for backend

3. 🔵 **REFACTOR**: Improve resilience
   - Add exponential backoff
   - Log export failures
   - Add health check

**Acceptance Criteria**:

- [ ] Application continues if tracing fails
- [ ] Export failures logged but don't crash
- [ ] Retry logic prevents thundering herd

---

### Phase 5.2: Documentation & Examples

**Goal**: Comprehensive documentation and examples

**TDD Workflow**:

1. 🔴 **RED**: Write documentation tests

   - Test: Example configurations compile
   - Test: Code examples in docs work

2. 🟢 **GREEN**: Write documentation

   - Add module-level docs
   - Create `examples/tracing_jaeger.rs`
   - Create `examples/tracing_tempo.rs`
   - Update README.md

3. 🔵 **REFACTOR**: Improve docs
   - Add architecture diagrams
   - Add troubleshooting guide

**Acceptance Criteria**:

- [ ] All public APIs documented
- [ ] Examples run successfully
- [ ] Troubleshooting guide complete

---

## ⏳ PLANNED: Phase 1 - Core Infrastructure

### Phase 1.1: HTTP Server with Pingora Framework

**Goal**: Replace placeholder server with actual Pingora-based HTTP server

**Dependencies**:

- Add `pingora` and `pingora-core` to Cargo.toml
- Study Yatagarasu implementation for reference

**TDD Workflow**:

1. 🔴 **RED**: Write failing test for basic HTTP server startup

   - Test: Server binds to configured address
   - Test: Server responds to health check endpoint
   - **PR #18**: RED phase - failing tests

2. 🟢 **GREEN**: Implement minimal Pingora server

   - Create `src/server/pingora_service.rs`
   - Implement `ProxyHttp` trait for S3 proxy
   - Basic request/response handling
   - **PR #19**: GREEN phase - passing tests

3. 🔵 **REFACTOR**: Clean up server code
   - Extract common patterns
   - Improve error handling
   - Add documentation
   - **PR #20**: REFACTOR phase - improved code

**Files to Create/Modify**:

- `src/server/pingora_service.rs` (new)
- `src/server/mod.rs` (modify)
- `tests/server_test.rs` (new)

**Acceptance Criteria**:

- [ ] Server starts and binds to configured port
- [ ] Server handles basic HTTP requests
- [ ] Graceful shutdown on SIGTERM/SIGINT
- [ ] All tests pass: `cargo test --lib server`

---

### Phase 1.2: Request Router Enhancement

**Goal**: Complete S3 request routing with bucket resolution

**TDD Workflow**:

1. 🔴 **RED**: Write tests for bucket-to-S3 mapping

   - Test: Route `/uploads/file.txt` to correct S3 bucket
   - Test: Reject requests to non-configured buckets
   - **PR #21**: RED phase

2. 🟢 **GREEN**: Implement bucket resolver

   - Create `BucketResolver` struct
   - Map path prefixes to S3 configs
   - **PR #22**: GREEN phase

3. 🔵 **REFACTOR**: Optimize routing
   - Use trie or hashmap for fast lookup
   - Add caching if needed
   - **PR #23**: REFACTOR phase

**Files to Create/Modify**:

- `src/router/bucket_resolver.rs` (new)
- `src/router/mod.rs` (modify)
- `tests/router_test.rs` (modify)

**Acceptance Criteria**:

- [ ] Bucket resolution works correctly
- [ ] Invalid buckets rejected with 404
- [ ] All tests pass

---

### Phase 1.3: S3 Client Integration

**Goal**: Integrate AWS SDK for actual S3 operations

**TDD Workflow**:

1. 🔴 **RED**: Write tests for S3 client

   - Test: Client initializes with credentials
   - Test: Client can sign requests
   - Test: Connection pooling works
   - **PR #24**: RED phase

2. 🟢 **GREEN**: Implement S3 client

   - Add `aws-sdk-s3` dependency
   - Create `S3ClientPool` struct
   - Implement credential loading
   - **PR #25**: GREEN phase

3. 🔵 **REFACTOR**: Optimize client
   - Add connection pooling
   - Add retry logic
   - Add timeout configuration
   - **PR #26**: REFACTOR phase

**Files to Create/Modify**:

- `src/s3/client.rs` (modify)
- `src/s3/credentials.rs` (new)
- `tests/s3_client_test.rs` (new)

**Acceptance Criteria**:

- [ ] S3 client connects to AWS
- [ ] Credentials loaded correctly
- [ ] Connection pooling works
- [ ] All tests pass

---

## ⏳ PLANNED: Phase 2 - Upload Operations

### Phase 2.1: Simple PutObject Handler

**Goal**: Implement single-part upload for files ≤50MB

**TDD Workflow**:

1. 🔴 **RED**: Write tests for PutObject

   - Test: Upload small file (1MB)
   - Test: Upload medium file (50MB)
   - Test: Handle upload errors
   - **PR #27**: RED phase

2. 🟢 **GREEN**: Implement PutObject handler

   - Create upload handler in `src/upload/put_object.rs`
   - Stream request body to S3
   - Return appropriate response
   - **PR #28**: GREEN phase

3. 🔵 **REFACTOR**: Optimize upload
   - Add progress tracking
   - Add metrics
   - Improve error handling
   - **PR #29**: REFACTOR phase

**Files to Create/Modify**:

- `src/upload/put_object.rs` (modify)
- `tests/put_object_test.rs` (new)

**Acceptance Criteria**:

- [ ] Files ≤50MB upload successfully
- [ ] Errors handled gracefully
- [ ] Metrics recorded
- [ ] All tests pass

---

### Phase 2.2: Multipart Upload Handler

**Goal**: Implement multipart upload for files >50MB

**TDD Workflow**:

1. 🔴 **RED**: Write tests for multipart upload

   - Test: Create multipart upload
   - Test: Upload parts
   - Test: Complete multipart upload
   - Test: Abort multipart upload
   - **PR #30**: RED phase

2. 🟢 **GREEN**: Implement multipart handler

   - Implement CreateMultipartUpload
   - Implement UploadPart
   - Implement CompleteMultipartUpload
   - Implement AbortMultipartUpload
   - **PR #31**: GREEN phase

3. 🔵 **REFACTOR**: Optimize multipart
   - Add concurrent part uploads
   - Add part retry logic
   - Add progress tracking
   - **PR #32**: REFACTOR phase

**Files to Create/Modify**:

- `src/upload/multipart.rs` (modify)
- `tests/multipart_test.rs` (new)

**Acceptance Criteria**:

- [ ] Files >50MB upload successfully
- [ ] Concurrent part uploads work
- [ ] Failed parts retried
- [ ] All tests pass

---

### Phase 2.3: Zero-Copy Integration

**Goal**: Integrate zero-copy transfer into upload handlers

**TDD Workflow**:

1. 🔴 **RED**: Write tests for zero-copy

   - Test: Zero-copy used on Linux
   - Test: Fallback used on macOS/Windows
   - Test: Performance improvement measured
   - **PR #33**: RED phase

2. 🟢 **GREEN**: Integrate zero-copy

   - Use `DataTransfer` in upload handlers
   - Add platform detection
   - Stream with splice/sendfile on Linux
   - **PR #34**: GREEN phase

3. 🔵 **REFACTOR**: Optimize zero-copy
   - Tune pipe buffer size
   - Add metrics for zero-copy usage
   - Benchmark performance
   - **PR #35**: REFACTOR phase

**Files to Create/Modify**:

- `src/upload/put_object.rs` (modify)
- `src/upload/multipart.rs` (modify)
- `benches/zero_copy_benchmark.rs` (new)

**Acceptance Criteria**:

- [ ] Zero-copy works on Linux
- [ ] Fallback works on other platforms
- [ ] 50-250x speedup on Linux
- [ ] All tests pass

---

## ⏳ PLANNED: Phase 3 - Authentication

### Phase 3.1: JWT Authentication

**Goal**: Implement JWT token validation

**TDD Workflow**:

1. 🔴 **RED**: Write tests for JWT validation

   - Test: Valid JWT accepted
   - Test: Invalid JWT rejected
   - Test: Expired JWT rejected
   - Test: JWKS endpoint support
   - **PR #36**: RED phase

2. 🟢 **GREEN**: Implement JWT validator

   - Add JWT validation logic
   - Support HS256/RS256/ES256
   - Add JWKS client
   - **PR #37**: GREEN phase

3. 🔵 **REFACTOR**: Optimize JWT
   - Cache JWKS keys
   - Add key rotation support
   - **PR #38**: REFACTOR phase

**Files to Create/Modify**:

- `src/auth/jwt.rs` (modify)
- `src/auth/jwks.rs` (new)
- `tests/jwt_test.rs` (new)

**Acceptance Criteria**:

- [ ] JWT validation works
- [ ] JWKS endpoint supported
- [ ] All tests pass

---

### Phase 3.2: AWS SigV4 Authentication

**Goal**: Implement AWS Signature Version 4 validation

**TDD Workflow**:

1. 🔴 **RED**: Write tests for SigV4

   - Test: Valid signature accepted
   - Test: Invalid signature rejected
   - Test: Replay attack prevented
   - **PR #39**: RED phase

2. 🟢 **GREEN**: Implement SigV4 validator

   - Add signature validation
   - Add timestamp validation
   - Add credential lookup
   - **PR #40**: GREEN phase

3. 🔵 **REFACTOR**: Optimize SigV4
   - Cache credentials
   - Add signature caching
   - **PR #41**: REFACTOR phase

**Files to Create/Modify**:

- `src/auth/sigv4.rs` (modify)
- `tests/sigv4_test.rs` (new)

**Acceptance Criteria**:

- [ ] SigV4 validation works
- [ ] Replay attacks prevented
- [ ] All tests pass

---

## ⏳ PLANNED: Phase 4 - Authorization

### Phase 4.1: OPA Integration

**Goal**: Integrate Open Policy Agent for authorization

**TDD Workflow**:

1. 🔴 **RED**: Write tests for OPA

   - Test: Policy evaluation works
   - Test: Allow/deny decisions enforced
   - **PR #42**: RED phase

2. 🟢 **GREEN**: Implement OPA client

   - Add OPA HTTP client
   - Add policy evaluation
   - **PR #43**: GREEN phase

3. 🔵 **REFACTOR**: Optimize OPA
   - Add caching
   - Add connection pooling
   - **PR #44**: REFACTOR phase

**Files to Create/Modify**:

- `src/authz/opa/mod.rs` (modify)
- `tests/opa_test.rs` (new)

**Acceptance Criteria**:

- [ ] OPA integration works
- [ ] Policies enforced correctly
- [ ] All tests pass

---

### Phase 4.2: OpenFGA Integration

**Goal**: Integrate OpenFGA for fine-grained authorization

**TDD Workflow**:

1. 🔴 **RED**: Write tests for OpenFGA

   - Test: Relationship checks work
   - Test: Authorization decisions correct
   - **PR #45**: RED phase

2. 🟢 **GREEN**: Implement OpenFGA client

   - Add OpenFGA gRPC client
   - Add relationship checks
   - **PR #46**: GREEN phase

3. 🔵 **REFACTOR**: Optimize OpenFGA
   - Add caching
   - Add batch checks
   - **PR #47**: REFACTOR phase

**Files to Create/Modify**:

- `src/authz/openfga/mod.rs` (modify)
- `tests/openfga_test.rs` (new)

**Acceptance Criteria**:

- [ ] OpenFGA integration works
- [ ] Fine-grained authz works
- [ ] All tests pass

---

- [ ] Files ≤50MB upload successfully
- [ ] Errors handled gracefully
- [ ] Metrics recorded
- [ ] All tests pass

---

### Phase 5.2: Documentation & Examples

**Goal**: Comprehensive documentation and examples

**TDD Workflow**:

1. 🔴 **RED**: Write documentation tests

   - Test: Example configurations compile
   - Test: Code examples in docs work

2. 🟢 **GREEN**: Write documentation

   - Add module-level docs
   - Create `examples/tracing_jaeger.rs`
   - Create `examples/tracing_tempo.rs`
   - Update README.md

3. 🔵 **REFACTOR**: Improve docs
   - Add architecture diagrams
   - Add troubleshooting guide

**Acceptance Criteria**:

- [ ] All public APIs documented
- [ ] Examples run successfully
- [ ] Troubleshooting guide complete

---

## ⏳ PLANNED: Phase 5 - Production Readiness

### Phase 5.1: Metrics & Monitoring

**Goal**: Production-ready metrics and monitoring

**TDD Workflow**:

1. 🔴 **RED**: Write tests for metrics

   - Test: Prometheus metrics exposed
   - Test: Upload metrics recorded
   - Test: Error metrics recorded
   - **PR #48**: RED phase

2. 🟢 **GREEN**: Implement metrics

   - Add Prometheus HTTP server
   - Add upload counters/histograms
   - Add error counters
   - **PR #49**: GREEN phase

3. 🔵 **REFACTOR**: Optimize metrics
   - Add custom metrics
   - Add metric labels
   - **PR #50**: REFACTOR phase

**Files to Create/Modify**:

- `src/metrics/server.rs` (new)
- `src/metrics/mod.rs` (modify)

**Acceptance Criteria**:

- [ ] Metrics HTTP server works
- [ ] All key metrics recorded
- [ ] All tests pass

---

### Phase 5.2: End-to-End Testing

**Goal**: Comprehensive E2E tests

**TDD Workflow**:

1. 🔴 **RED**: Write E2E tests

   - Test: Full upload flow
   - Test: Auth + upload flow
   - Test: Error scenarios
   - **PR #51**: RED phase

2. 🟢 **GREEN**: Implement E2E tests

   - Create E2E test framework
   - Add integration tests
   - **PR #52**: GREEN phase

3. 🔵 **REFACTOR**: Improve E2E tests
   - Add performance tests
   - Add load tests
   - **PR #53**: REFACTOR phase

**Files to Create/Modify**:

- `tests/e2e/` (new directory)
- `tests/e2e/upload_test.rs` (new)

**Acceptance Criteria**:

- [ ] E2E tests cover all flows
- [ ] All tests pass
- [ ] Performance benchmarks pass

---

## 🎯 Milestones

### Milestone 1: Tracing Complete ✅ 75% DONE

- **Target**: 2025-12-31
- **Status**: 🚧 In Progress
- **Remaining**: Phase 4.2-4.4, Phase 5
- **Deliverable**: Production-ready distributed tracing

### Milestone 2: Core Infrastructure

- **Target**: 2026-01-15
- **Status**: ⏳ Not Started
- **Tasks**: Phase 1.1-1.3
- **Deliverable**: Pingora server, routing, S3 client

### Milestone 3: Upload Operations

- **Target**: 2026-02-01
- **Status**: ⏳ Not Started
- **Tasks**: Phase 2.1-2.3
- **Deliverable**: PutObject, multipart, zero-copy

### Milestone 4: Auth & AuthZ

- **Target**: 2026-02-15
- **Status**: ⏳ Not Started
- **Tasks**: Phase 3.1-3.2, Phase 4.1-4.2
- **Deliverable**: JWT, SigV4, OPA, OpenFGA

### Milestone 5: Production Ready

- **Target**: 2026-03-01
- **Status**: ⏳ Not Started
- **Tasks**: Phase 5.1-5.2
- **Deliverable**: Metrics, E2E tests, documentation

---

## 📋 Quality Checklist (Per PR)

Each PR must satisfy:

- [ ] **All tests pass**: `cargo test --all-features`
- [ ] **No Clippy warnings**: `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] **Code formatted**: `cargo fmt --check`
- [ ] **Documentation updated**: `cargo doc --no-deps`
- [ ] **PR description includes**:
  - [ ] What was changed (RED/GREEN/REFACTOR)
  - [ ] Why it was changed
  - [ ] How to test it
  - [ ] Screenshots/traces (if applicable)

---

## 🚀 Development Workflow

### Prerequisites

1. Install Rust toolchain (1.75+)
2. Start Jaeger for tracing:
   ```bash
   docker run -d --name jaeger \
     -p 4317:4317 -p 16686:16686 \
     jaegertracing/all-in-one:latest
   ```
3. Start MinIO for local S3:
   ```bash
   docker compose up -d
   ```

### TDD Workflow

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
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check

# 6. Create PR
git push origin feature/new-capability
# Create PR on GitHub

# 7. After review and approval, merge
```

### Testing Commands

```bash
# Build with tracing
cargo build --features tracing

# Run unit tests
cargo test --lib

# Run integration tests
cargo test --test '*'

# Run benchmarks
cargo bench

# Run E2E tests
python3 test_zero_copy.py

# Test with Jaeger
cargo run --features tracing -- --config config.yaml
# Upload a file, then check http://localhost:16686
```

---

## 🔗 References

### Internal Documents

- [CLAUDE.md](CLAUDE.md) - AI assistant context and TDD methodology
- [README.md](README.md) - Project overview and quick start
- [config.example.yaml](config.example.yaml) - Example configuration

### Sister Project

- [Yatagarasu](https://github.com/julianshen/yatagarasu) - Read-only S3 proxy (reference for auth/authz)

### External References

- [Linux splice(2)](https://man7.org/linux/man-pages/man2/splice.2.html)
- [AWS S3 REST API](https://docs.aws.amazon.com/AmazonS3/latest/API/)
- [Pingora Framework](https://github.com/cloudflare/pingora)
- [OpenTelemetry Rust](https://github.com/open-telemetry/opentelemetry-rust)
- [W3C Trace Context](https://www.w3.org/TR/trace-context/)

---

## 📝 Notes

### Architecture Decisions

1. **Upload-only by design** - No download/list operations for security
2. **Zero-copy on Linux** - 50-250x performance improvement
3. **Cross-platform fallback** - Works on macOS/Windows with buffered I/O
4. **50MB multipart threshold** - Automatic chunking for large files
5. **TDD methodology** - All features developed with Red-Green-Refactor

### Performance Expectations

| File Size | Linux (zero-copy) | macOS/Windows (buffered) |
| --------- | ----------------- | ------------------------ |
| 1 MB      | ~2 ms             | ~10 ms                   |
| 10 MB     | ~12 ms            | ~600 ms                  |
| 50 MB     | ~60 ms            | ~15,000 ms               |

### Key Dependencies

| Crate                | Purpose                     |
| -------------------- | --------------------------- |
| `tokio`              | Async runtime               |
| `pingora`            | HTTP server framework       |
| `aws-sdk-s3`         | S3 client                   |
| `nix`                | Linux syscalls (Linux only) |
| `jsonwebtoken`       | JWT validation              |
| `prometheus`         | Metrics                     |
| `opentelemetry`      | Distributed tracing         |
| `opentelemetry-otlp` | OTLP exporter               |

---

_Last Updated: 2025-12-25_
_Next Review: 2025-12-31_
_Maintained by: @julianshen_

- [ ] All public APIs documented
- [ ] Examples run successfully
- [ ] Troubleshooting guide complete

---
