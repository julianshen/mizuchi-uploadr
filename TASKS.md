# Mizuchi Uploadr - Task List

> **Generated from**: plan.md
> **Last Updated**: 2025-12-25
> **Methodology**: TDD Red-Green-Refactor

---

## 🎯 Priority Queue

### 🔴 IMMEDIATE (This Week)

#### Task 1: Tracing Phase 4.2 - W3C Trace Context Extraction

- **Status**: ⏳ Not Started
- **Priority**: HIGH
- **Estimated**: 2-3 days
- **Blocker**: None
- **Goal**: Extract traceparent from incoming HTTP requests
- **Impact**: Enables end-to-end distributed tracing
- **Files**:
  - `src/tracing/propagation.rs` (new)
  - `src/server/tracing_middleware.rs` (modify)
  - `tests/context_propagation_test.rs` (modify)
- **Subtasks**:
  - [ ] 🔴 RED: Write failing tests for trace context extraction
  - [ ] 🔴 RED: Test traceparent header extraction
  - [ ] 🔴 RED: Test W3C format parsing
  - [ ] 🔴 RED: Test invalid traceparent handling
  - [ ] 🟢 GREEN: Create `src/tracing/propagation.rs`
  - [ ] 🟢 GREEN: Implement traceparent/tracestate extraction
  - [ ] 🟢 GREEN: Parse W3C format (version-trace_id-span_id-flags)
  - [ ] 🟢 GREEN: Link spans correctly
  - [ ] 🔵 REFACTOR: Reduce parsing overhead
  - [ ] 🔵 REFACTOR: Add validation for trace context
  - [ ] 🔵 REFACTOR: Improve error handling
  - [ ] ✅ Verify: Trace context extracted from incoming requests
  - [ ] ✅ Verify: Distributed traces work end-to-end
  - [ ] ✅ Verify: Invalid trace context handled gracefully
  - [ ] ✅ Verify: All tests pass

#### Task 2: Tracing Phase 4.3 - Auth/AuthZ Tracing

- **Status**: ⏳ Not Started
- **Priority**: HIGH
- **Estimated**: 2-3 days
- **Depends On**: Task 1
- **Goal**: Add tracing to authentication and authorization operations
- **Impact**: Complete observability for security operations
- **Files**:
  - `src/auth/jwt.rs` (modify)
  - `src/auth/sigv4.rs` (modify)
  - `src/authz/opa/mod.rs` (modify)
  - `src/authz/openfga/mod.rs` (modify)
  - `tests/auth_tracing_test.rs` (new)
- **Subtasks**:
  - [ ] 🔴 RED: Write failing tests for JWT validation span
  - [ ] 🔴 RED: Write failing tests for SigV4 validation span
  - [ ] 🔴 RED: Write failing tests for OPA/OpenFGA tracing
  - [ ] 🔴 RED: Test no PII in span attributes
  - [ ] 🟢 GREEN: Add #[instrument] to JWT validators
  - [ ] 🟢 GREEN: Add #[instrument] to SigV4 validators
  - [ ] 🟢 GREEN: Add spans to OPA client
  - [ ] 🟢 GREEN: Add spans to OpenFGA client
  - [ ] 🔵 REFACTOR: Add user ID (hashed, no PII)
  - [ ] 🔵 REFACTOR: Add auth method used
  - [ ] 🔵 REFACTOR: Add authorization decision (allow/deny)
  - [ ] ✅ Verify: Auth operations traced (no PII)
  - [ ] ✅ Verify: Authorization decisions logged
  - [ ] ✅ Verify: All tests pass

#### Task 3: Tracing Phase 4.4 - Performance Optimization

- **Status**: ⏳ Not Started
- **Priority**: MEDIUM
- **Estimated**: 2-3 days
- **Depends On**: Task 2
- **Goal**: Optimize tracing performance and measure overhead
- **Impact**: Ensure tracing overhead < 5%
- **Files**:
  - `benches/tracing_benchmark.rs` (new)
  - `src/tracing/init.rs` (modify)
- **Subtasks**:
  - [ ] 🔴 RED: Benchmark request with tracing vs without
  - [ ] 🔴 RED: Benchmark span creation overhead
  - [ ] 🔴 RED: Benchmark OTLP export latency
  - [ ] 🟢 GREEN: Reduce tracing overhead
  - [ ] 🟢 GREEN: Optimize span creation
  - [ ] 🟢 GREEN: Tune batch export settings
  - [ ] 🔵 REFACTOR: Profile and optimize hot paths
  - [ ] 🔵 REFACTOR: Reduce allocations
  - [ ] ✅ Verify: Sampling strategies configurable
  - [ ] ✅ Verify: Performance overhead < 5%
  - [ ] ✅ Verify: Benchmarks show acceptable overhead

#### Task 4: Tracing Phase 5.1 - Error Handling & Resilience

- **Status**: ⏳ Not Started
- **Priority**: MEDIUM
- **Estimated**: 2-3 days
- **Depends On**: Task 3
- **Goal**: Ensure application continues if tracing fails
- **Impact**: Production-ready error handling
- **Files**:
  - `src/tracing/error.rs` (new)
- **Subtasks**:
  - [ ] 🔴 RED: Test OTLP backend unavailable
  - [ ] 🔴 RED: Test network timeout
  - [ ] 🔴 RED: Test application continues on tracing failure
  - [ ] 🟢 GREEN: Create `src/tracing/error.rs`
  - [ ] 🟢 GREEN: Add retry logic for OTLP export
  - [ ] 🟢 GREEN: Fallback to console logging
  - [ ] 🟢 GREEN: Circuit breaker for backend
  - [ ] 🔵 REFACTOR: Add exponential backoff
  - [ ] 🔵 REFACTOR: Log export failures
  - [ ] 🔵 REFACTOR: Add health check
  - [ ] ✅ Verify: Application continues if tracing fails
  - [ ] ✅ Verify: Export failures logged but don't crash
  - [ ] ✅ Verify: Retry logic prevents thundering herd

#### Task 5: Tracing Phase 5.2 - Documentation & Examples

- **Status**: ⏳ Not Started
- **Priority**: MEDIUM
- **Estimated**: 2-3 days
- **Depends On**: Task 4
- **Goal**: Comprehensive documentation and examples
- **Impact**: Production-ready documentation
- **Files**:
  - `examples/tracing_jaeger.rs` (new)
  - `examples/tracing_tempo.rs` (new)
  - `docs/TRACING.md` (new)
- **Subtasks**:
  - [ ] 🔴 RED: Test example configurations compile
  - [ ] 🔴 RED: Test code examples in docs work
  - [ ] 🟢 GREEN: Add module-level docs
  - [ ] 🟢 GREEN: Create `examples/tracing_jaeger.rs`
  - [ ] 🟢 GREEN: Create `examples/tracing_tempo.rs`
  - [ ] 🟢 GREEN: Update README.md
  - [ ] 🔵 REFACTOR: Add architecture diagrams
  - [ ] 🔵 REFACTOR: Add troubleshooting guide
  - [ ] ✅ Verify: All public APIs documented
  - [ ] ✅ Verify: Examples run successfully
  - [ ] ✅ Verify: Troubleshooting guide complete

---

### 🟡 SHORT-TERM (Next 2 Weeks)

#### Task 6: Phase 1.1 - HTTP Server with Pingora Framework

- **Status**: ✅ **COMPLETE** (2025-12-26)
- **Priority**: HIGH
- **Estimated**: 1 week
- **Actual**: 1 day
- **Blocker**: Tracing must be complete (Tasks 1-5)
- **Goal**: Replace placeholder server with actual Pingora-based HTTP server
- **Impact**: Core HTTP functionality
- **Files**:
  - `src/server/pingora.rs` (created)
  - `src/server/mod.rs` (modified)
  - `tests/pingora_server_test.rs` (created)
- **PRs**: #21 (RED - merged), #22 (GREEN - merged), #23 (REFACTOR - merged)
- **Subtasks**:
  - [x] 🔴 RED: Test server binds to configured address
  - [x] 🔴 RED: Test server responds to health check endpoint
  - [x] 🟢 GREEN: Create `src/server/pingora.rs`
  - [x] 🟢 GREEN: Implement HTTP server with hyper
  - [x] 🟢 GREEN: Basic request/response handling
  - [x] 🔵 REFACTOR: Extract common patterns
  - [x] 🔵 REFACTOR: Improve error handling
  - [x] 🔵 REFACTOR: Add comprehensive documentation
  - [x] 🔵 REFACTOR: Fix all clippy warnings
  - [x] ✅ Verify: Server starts and binds to configured port
  - [x] ✅ Verify: Server handles basic HTTP requests
  - [x] ✅ Verify: Graceful shutdown on SIGTERM/SIGINT
  - [x] ✅ Verify: All tests pass

#### Task 7: Phase 1.2 - Request Router Enhancement

- **Status**: ✅ **COMPLETE** (2025-12-26)
- **Priority**: HIGH
- **Estimated**: 3-4 days
- **Actual**: 1 day
- **Depends On**: Task 6 ✅
- **Goal**: Complete S3 request routing with bucket resolution
- **Impact**: Multi-bucket support
- **Files**:
  - `src/router/bucket_resolver.rs` (created)
  - `src/router/mod.rs` (modified)
  - `tests/bucket_resolver_test.rs` (created)
- **PRs**: #24 (RED - merged), #25 (GREEN+REFACTOR - merged)
- **Subtasks**:
  - [x] 🔴 RED: Test route `/uploads/file.txt` to correct S3 bucket
  - [x] 🔴 RED: Test reject requests to non-configured buckets
  - [x] 🟢 GREEN: Create `BucketResolver` struct
  - [x] 🟢 GREEN: Map path prefixes to S3 configs
  - [x] 🔵 REFACTOR: Use HashMap for fast O(1) lookup
  - [x] ✅ Verify: Bucket resolution works correctly
  - [x] ✅ Verify: Invalid buckets rejected with 404
  - [x] ✅ Verify: All tests pass

#### Task 8: Phase 1.3 - S3 Client Integration

- **Status**: ✅ **COMPLETE** (2025-12-27)
- **Priority**: HIGH
- **Estimated**: 1 week
- **Actual**: 1 day
- **Depends On**: Task 7 ✅
- **Goal**: Integrate AWS SDK for actual S3 operations
- **Impact**: Real S3 connectivity
- **Files**:
  - `src/s3/mod.rs` (modified - added SigV4 signing, retry, timeout)
  - `src/s3/credentials.rs` (created)
  - `src/s3/pool.rs` (created)
  - `tests/s3_client_pool_test.rs` (created)
- **PRs**: #26 (RED+GREEN - merged), #27 (REFACTOR)
- **Subtasks**:
  - [x] 🔴 RED: Test client initializes with credentials
  - [x] 🔴 RED: Test client can sign requests
  - [x] 🔴 RED: Test connection pooling works
  - [x] 🟢 GREEN: Add `aws-smithy-runtime-api` dependency
  - [x] 🟢 GREEN: Create `S3ClientPool` struct
  - [x] 🟢 GREEN: Implement credential loading (CredentialsProvider trait)
  - [x] 🟢 GREEN: Add SigV4 signing to S3Client
  - [x] 🔵 REFACTOR: Add retry logic with exponential backoff (RetryConfig)
  - [x] 🔵 REFACTOR: Add timeout configuration (TimeoutConfig)
  - [x] 🔵 REFACTOR: Add x-amz-content-sha256 header for S3 signing
  - [x] 🔵 REFACTOR: Add unit tests for retry/timeout/backoff logic
  - [x] ✅ Verify: S3 client connects with credentials
  - [x] ✅ Verify: Credentials loaded correctly
  - [x] ✅ Verify: Connection pooling works
  - [x] ✅ Verify: All 79 tests pass (49 unit + 30 integration)

---

### 🟢 MEDIUM-TERM (Next Month)

#### Task 9: Phase 2.1 - Simple PutObject Handler

- **Status**: ✅ **COMPLETE** (2025-12-27)
- **Priority**: MEDIUM
- **Estimated**: 1 week
- **Actual**: 1 hour
- **Depends On**: Task 8 ✅
- **Goal**: Implement single-part upload for files ≤50MB
- **Impact**: Basic upload functionality
- **Files**:
  - `src/upload/put_object.rs` (modified - connected to S3Client)
  - `tests/put_object_handler_test.rs` (created)
- **PRs**: #28 (RED+GREEN+REFACTOR)
- **Subtasks**:
  - [x] 🔴 RED: Test upload small file (1KB, 1MB)
  - [x] 🔴 RED: Test real ETag returned (not fake UUID)
  - [x] 🔴 RED: Test handle upload errors (403, 500)
  - [x] 🔴 RED: Test Content-Type preservation
  - [x] 🔴 RED: Test body integrity
  - [x] 🟢 GREEN: Add `with_client()` constructor accepting S3Client
  - [x] 🟢 GREEN: Call S3Client.put_object() in upload handler
  - [x] 🟢 GREEN: Return real ETag from S3 response
  - [x] 🔵 REFACTOR: Add Prometheus metrics (uploads_total, bytes, duration)
  - [x] 🔵 REFACTOR: Add timing and duration logging
  - [x] ✅ Verify: Files upload successfully via S3Client
  - [x] ✅ Verify: Errors handled gracefully with metrics
  - [x] ✅ Verify: All 7 integration tests pass
  - [x] ✅ Verify: All 56 total tests pass

#### Task 10: Phase 2.2 - Multipart Upload Handler

- **Status**: ⏳ Not Started
- **Priority**: MEDIUM
- **Estimated**: 1 week
- **Depends On**: Task 9
- **Goal**: Implement multipart upload for files >50MB
- **Impact**: Large file support
- **Files**:
  - `src/upload/multipart.rs` (modify)
  - `tests/multipart_test.rs` (new)
- **PRs**: #30 (RED), #31 (GREEN), #32 (REFACTOR)
- **Subtasks**:
  - [ ] 🔴 RED: Test create multipart upload
  - [ ] 🔴 RED: Test upload parts
  - [ ] 🔴 RED: Test complete multipart upload
  - [ ] 🔴 RED: Test abort multipart upload
  - [ ] 🟢 GREEN: Implement CreateMultipartUpload
  - [ ] 🟢 GREEN: Implement UploadPart
  - [ ] 🟢 GREEN: Implement CompleteMultipartUpload
  - [ ] 🟢 GREEN: Implement AbortMultipartUpload
  - [ ] 🔵 REFACTOR: Add concurrent part uploads
  - [ ] 🔵 REFACTOR: Add part retry logic
  - [ ] 🔵 REFACTOR: Add progress tracking
  - [ ] ✅ Verify: Files >50MB upload successfully
  - [ ] ✅ Verify: Concurrent part uploads work
  - [ ] ✅ Verify: Failed parts retried
  - [ ] ✅ Verify: All tests pass

#### Task 11: Phase 2.3 - Zero-Copy Integration

- **Status**: ⏳ Not Started
- **Priority**: MEDIUM
- **Estimated**: 1 week
- **Depends On**: Task 10
- **Goal**: Integrate zero-copy transfer into upload handlers
- **Impact**: 50-250x performance improvement on Linux
- **Files**:
  - `src/upload/put_object.rs` (modify)
  - `src/upload/multipart.rs` (modify)
  - `benches/zero_copy_benchmark.rs` (new)
- **PRs**: #33 (RED), #34 (GREEN), #35 (REFACTOR)
- **Subtasks**:

  - [ ] 🔴 RED: Test zero-copy used on Linux
  - [ ] 🔴 RED: Test fallback used on macOS/Windows
  - [ ] 🔴 RED: Test performance improvement measured
  - [ ] 🟢 GREEN: Use `DataTransfer` in upload handlers
  - [ ] 🟢 GREEN: Add platform detection
  - [ ] 🟢 GREEN: Stream with splice/sendfile on Linux
  - [ ] 🔵 REFACTOR: Tune pipe buffer size
  - [ ] 🔵 REFACTOR: Add metrics for zero-copy usage
  - [ ] 🔵 REFACTOR: Benchmark performance
  - [ ] ✅ Verify: Zero-copy works on Linux
  - [ ] ✅ Verify: Fallback works on other platforms
  - [ ] ✅ Verify: 50-250x speedup on Linux
  - [ ] ✅ Verify: All tests pass

---

### 🔵 LONG-TERM (Future)

#### Task 12: Phase 3.1 - JWT Authentication

- **Status**: ⏳ Not Started
- **Priority**: LOW
- **Estimated**: 1 week
- **Depends On**: Task 11
- **Goal**: Implement JWT token validation
- **Impact**: JWT authentication support
- **Files**:
  - `src/auth/jwt.rs` (modify)
  - `src/auth/jwks.rs` (new)
  - `tests/jwt_test.rs` (new)
- **PRs**: #36 (RED), #37 (GREEN), #38 (REFACTOR)
- **Subtasks**:
  - [ ] 🔴 RED: Test valid JWT accepted
  - [ ] 🔴 RED: Test invalid JWT rejected
  - [ ] 🔴 RED: Test expired JWT rejected
  - [ ] 🔴 RED: Test JWKS endpoint support
  - [ ] 🟢 GREEN: Add JWT validation logic
  - [ ] 🟢 GREEN: Support HS256/RS256/ES256
  - [ ] 🟢 GREEN: Add JWKS client
  - [ ] 🔵 REFACTOR: Cache JWKS keys
  - [ ] 🔵 REFACTOR: Add key rotation support
  - [ ] ✅ Verify: JWT validation works
  - [ ] ✅ Verify: JWKS endpoint supported
  - [ ] ✅ Verify: All tests pass

#### Task 13: Phase 3.2 - AWS SigV4 Authentication

- **Status**: ⏳ Not Started
- **Priority**: LOW
- **Estimated**: 1 week
- **Depends On**: Task 12
- **Goal**: Implement AWS Signature Version 4 validation
- **Impact**: AWS SigV4 authentication support
- **Files**:
  - `src/auth/sigv4.rs` (modify)
  - `tests/sigv4_test.rs` (new)
- **PRs**: #39 (RED), #40 (GREEN), #41 (REFACTOR)
- **Subtasks**:
  - [ ] 🔴 RED: Test valid signature accepted
  - [ ] 🔴 RED: Test invalid signature rejected
  - [ ] 🔴 RED: Test replay attack prevented
  - [ ] 🟢 GREEN: Add signature validation
  - [ ] 🟢 GREEN: Add timestamp validation
  - [ ] 🟢 GREEN: Add credential lookup
  - [ ] 🔵 REFACTOR: Cache credentials
  - [ ] 🔵 REFACTOR: Add signature caching
  - [ ] ✅ Verify: SigV4 validation works
  - [ ] ✅ Verify: Replay attacks prevented
  - [ ] ✅ Verify: All tests pass

#### Task 14: Phase 4.1 - OPA Integration

- **Status**: ⏳ Not Started
- **Priority**: LOW
- **Estimated**: 1 week
- **Depends On**: Task 13
- **Goal**: Integrate Open Policy Agent for authorization
- **Impact**: Policy-based authorization
- **Files**:
  - `src/authz/opa/mod.rs` (modify)
  - `tests/opa_test.rs` (new)
- **PRs**: #42 (RED), #43 (GREEN), #44 (REFACTOR)
- **Subtasks**:
  - [ ] 🔴 RED: Test policy evaluation works
  - [ ] 🔴 RED: Test allow/deny decisions enforced
  - [ ] 🟢 GREEN: Add OPA HTTP client
  - [ ] 🟢 GREEN: Add policy evaluation
  - [ ] 🔵 REFACTOR: Add caching
  - [ ] 🔵 REFACTOR: Add connection pooling
  - [ ] ✅ Verify: OPA integration works
  - [ ] ✅ Verify: Policies enforced correctly
  - [ ] ✅ Verify: All tests pass

#### Task 15: Phase 4.2 - OpenFGA Integration

- **Status**: ⏳ Not Started
- **Priority**: LOW
- **Estimated**: 1 week
- **Depends On**: Task 14
- **Goal**: Integrate OpenFGA for fine-grained authorization
- **Impact**: Fine-grained authorization support
- **Files**:
  - `src/authz/openfga/mod.rs` (modify)
  - `tests/openfga_test.rs` (new)
- **PRs**: #45 (RED), #46 (GREEN), #47 (REFACTOR)
- **Subtasks**:
  - [ ] 🔴 RED: Test relationship checks work
  - [ ] 🔴 RED: Test authorization decisions correct
  - [ ] 🟢 GREEN: Add OpenFGA gRPC client
  - [ ] 🟢 GREEN: Add relationship checks
  - [ ] 🔵 REFACTOR: Add caching
  - [ ] 🔵 REFACTOR: Add batch checks
  - [ ] ✅ Verify: OpenFGA integration works
  - [ ] ✅ Verify: Fine-grained authz works
  - [ ] ✅ Verify: All tests pass

#### Task 16: Phase 5.1 - Metrics & Monitoring

- **Status**: ⏳ Not Started
- **Priority**: LOW
- **Estimated**: 1 week
- **Depends On**: Task 15
- **Goal**: Production-ready metrics and monitoring
- **Impact**: Observability and monitoring
- **Files**:
  - `src/metrics/server.rs` (new)
  - `src/metrics/mod.rs` (modify)
- **PRs**: #48 (RED), #49 (GREEN), #50 (REFACTOR)
- **Subtasks**:
  - [ ] 🔴 RED: Test Prometheus metrics exposed
  - [ ] 🔴 RED: Test upload metrics recorded
  - [ ] 🔴 RED: Test error metrics recorded
  - [ ] 🟢 GREEN: Add Prometheus HTTP server
  - [ ] 🟢 GREEN: Add upload counters/histograms
  - [ ] 🟢 GREEN: Add error counters
  - [ ] 🔵 REFACTOR: Add custom metrics
  - [ ] 🔵 REFACTOR: Add metric labels
  - [ ] ✅ Verify: Metrics HTTP server works
  - [ ] ✅ Verify: All key metrics recorded
  - [ ] ✅ Verify: All tests pass

#### Task 17: Phase 5.2 - End-to-End Testing

- **Status**: ⏳ Not Started
- **Priority**: LOW
- **Estimated**: 1 week
- **Depends On**: Task 16
- **Goal**: Comprehensive E2E tests
- **Impact**: Production-ready testing
- **Files**:
  - `tests/e2e/` (new directory)
  - `tests/e2e/upload_test.rs` (new)
- **PRs**: #51 (RED), #52 (GREEN), #53 (REFACTOR)
- **Subtasks**:
  - [ ] 🔴 RED: Test full upload flow
  - [ ] 🔴 RED: Test auth + upload flow
  - [ ] 🔴 RED: Test error scenarios
  - [ ] 🟢 GREEN: Create E2E test framework
  - [ ] 🟢 GREEN: Add integration tests
  - [ ] 🔵 REFACTOR: Add performance tests
  - [ ] 🔵 REFACTOR: Add load tests
  - [ ] ✅ Verify: E2E tests cover all flows
  - [ ] ✅ Verify: All tests pass
  - [ ] ✅ Verify: Performance benchmarks pass

---

## 📊 Progress Summary

### Overall Status

- **Total Tasks**: 17
- **Completed**: 3 (Task 6: HTTP Server ✅, Task 7: Bucket Resolver ✅, Task 8: S3 Client ✅)
- **In Progress**: 0
- **Not Started**: 14
- **Total Estimated Time**: ~20 weeks
- **Time Saved**: Task 6 (6 days) + Task 7 (3 days) + Task 8 (6 days) = 15 days ahead!

### By Priority

- **🔴 HIGH Priority**: 5 tasks (Tasks 1-5: Tracing completion)
- **🟡 MEDIUM Priority**: 6 tasks (Tasks 6-11: Core infrastructure + uploads)
- **🔵 LOW Priority**: 6 tasks (Tasks 12-17: Auth/AuthZ + production)

### By Phase

- **Tracing (Phase 4-5)**: 5 tasks (Tasks 1-5)
- **Core Infrastructure (Phase 1)**: 3 tasks (Tasks 6-8)
- **Upload Operations (Phase 2)**: 3 tasks (Tasks 9-11)
- **Authentication (Phase 3)**: 2 tasks (Tasks 12-13)
- **Authorization (Phase 4)**: 2 tasks (Tasks 14-15)
- **Production Ready (Phase 5)**: 2 tasks (Tasks 16-17)

---

## 🎯 Milestones

### Milestone 1: Tracing Complete (Tasks 1-5)

- **Target**: 2025-12-31
- **Status**: ⏳ Not Started
- **Deliverable**: Production-ready distributed tracing

### Milestone 2: Core Infrastructure (Tasks 6-8)

- **Target**: 2026-01-15
- **Status**: ✅ **COMPLETE** (2025-12-27)
- **Progress**: 100% (3/3 tasks complete)
- **Deliverable**: Pingora server ✅, routing ✅, S3 client ✅

### Milestone 3: Upload Operations (Tasks 9-11)

- **Target**: 2026-02-01
- **Status**: 🚀 In Progress (Task 9 next)
- **Progress**: 0% (0/3 tasks complete)
- **Deliverable**: PutObject (next), multipart, zero-copy

### Milestone 4: Auth & AuthZ (Tasks 12-15)

- **Target**: 2026-02-15
- **Status**: ⏳ Not Started
- **Deliverable**: JWT, SigV4, OPA, OpenFGA

### Milestone 5: Production Ready (Tasks 16-17)

- **Target**: 2026-03-01
- **Status**: ⏳ Not Started
- **Deliverable**: Metrics, E2E tests, documentation

---

## 📝 Notes

### TDD Methodology

All tasks follow the Red-Green-Refactor cycle:

- 🔴 **RED**: Write failing tests first
- 🟢 **GREEN**: Implement minimal code to pass tests
- 🔵 **REFACTOR**: Clean up and optimize code
- ✅ **VERIFY**: Ensure all acceptance criteria met

### Quality Gates

Each task must pass:

- [ ] All tests pass: `cargo test --all-features`
- [ ] No Clippy warnings: `cargo clippy --all-targets --all-features -- -D warnings`
- [ ] Code formatted: `cargo fmt --check`
- [ ] Documentation updated: `cargo doc --no-deps`

### Dependencies

Tasks are ordered by dependency chain. Do not start a task until its dependencies are complete.

---

_Generated from plan.md on 2025-12-25_

---
