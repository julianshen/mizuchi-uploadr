# Mizuchi Uploadr - Implementation Plan

> **Methodology**: Strict TDD (Test-Driven Development) - Red-Green-Refactor Cycle
> **PR Strategy**: One PR per TDD phase (RED, GREEN, REFACTOR) for each feature

## Current Status

### ✅ Completed

- Project structure and scaffolding
- Configuration module (`src/config/`)
- Basic router with S3 operation parsing (`src/router/`)
- Zero-copy transfer abstraction (`src/upload/zero_copy.rs`)
- Metrics definitions (`src/metrics/`)
- Placeholder implementations for auth modules
- Basic integration tests

### 🚧 Partially Implemented

- Server module (placeholder, needs Pingora integration)
- S3 client (basic structure, needs actual AWS SDK integration)
- Upload handlers (placeholders only)
- Auth modules (stubs only)

### ❌ Not Started

- Actual HTTP request handling
- Real S3 upload operations
- Authentication implementation
- Authorization implementation
- Metrics HTTP server
- End-to-end testing

---

## Phase 1: Core Infrastructure (Foundation)

### 1.1 HTTP Server with Pingora Framework

**Goal**: Replace placeholder server with actual Pingora-based HTTP server

**Dependencies**:

- Add `pingora` and `pingora-core` to Cargo.toml
- Study Yatagarasu implementation for reference

**TDD Workflow**:

1. 🔴 **RED**: Write failing test for basic HTTP server startup

   - Test: Server binds to configured address
   - Test: Server responds to health check endpoint
   - **PR #1**: RED phase - failing tests

2. 🟢 **GREEN**: Implement minimal Pingora server

   - Create `src/server/pingora_service.rs`
   - Implement `ProxyHttp` trait for S3 proxy
   - Basic request/response handling
   - **PR #2**: GREEN phase - passing tests

3. 🔵 **REFACTOR**: Clean up server code
   - Extract common patterns
   - Improve error handling
   - Add documentation
   - **PR #3**: REFACTOR phase - improved code

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

### 1.2 Request Router Enhancement

**Goal**: Complete S3 request routing with bucket resolution

**TDD Workflow**:

1. 🔴 **RED**: Write tests for bucket-to-S3 mapping

   - Test: Route `/uploads/file.txt` to correct S3 bucket
   - Test: Reject requests to non-configured buckets
   - **PR #4**: RED phase

2. 🟢 **GREEN**: Implement bucket resolver

   - Create `BucketResolver` struct
   - Map path prefixes to S3 configs
   - **PR #5**: GREEN phase

3. 🔵 **REFACTOR**: Optimize routing logic
   - Use HashMap for O(1) bucket lookup
   - **PR #6**: REFACTOR phase

**Files to Create/Modify**:

- `src/router/bucket_resolver.rs` (new)
- `src/router/mod.rs` (modify)
- `tests/router_test.rs` (new)

**Acceptance Criteria**:

- [ ] Bucket resolution works for all configured buckets
- [ ] 404 for non-existent buckets
- [ ] Path prefix matching is correct
- [ ] All tests pass: `cargo test --lib router`

---

### 1.3 S3 Client Integration

**Goal**: Integrate AWS SDK for actual S3 operations

**TDD Workflow**:

1. 🔴 **RED**: Write tests for S3 client operations

   - Test: Create S3 client from config
   - Test: Generate presigned URLs
   - Test: Handle AWS credentials
   - **PR #7**: RED phase

2. 🟢 **GREEN**: Implement AWS SDK integration

   - Use `aws-sdk-s3` for operations
   - Implement credential providers
   - Connection pooling
   - **PR #8**: GREEN phase

3. 🔵 **REFACTOR**: Optimize client creation
   - Lazy initialization
   - Connection reuse
   - **PR #9**: REFACTOR phase

**Files to Create/Modify**:

- `src/s3/client.rs` (new)
- `src/s3/credentials.rs` (new)
- `src/s3/mod.rs` (modify)
- `tests/s3_client_test.rs` (new)

**Acceptance Criteria**:

- [ ] S3 client successfully connects to AWS/MinIO
- [ ] Credentials loaded from config/environment
- [ ] SigV4 signing works correctly
- [ ] All tests pass: `cargo test --lib s3`

---

## Phase 2: Upload Operations (Core Functionality)

### 2.1 Simple PutObject Handler

**Goal**: Implement single-part upload for files ≤50MB

**TDD Workflow**:

1. 🔴 **RED**: Write failing tests for PutObject

   - Test: Upload small file (1KB)
   - Test: Upload medium file (10MB)
   - Test: Verify ETag returned
   - Test: Content-Type preserved
   - **PR #10**: RED phase

2. 🟢 **GREEN**: Implement PutObject handler

   - Read request body
   - Call S3 PutObject API
   - Return upload result
   - **PR #11**: GREEN phase

3. 🔵 **REFACTOR**: Optimize body handling
   - Stream body without full buffering
   - Add timeout handling
   - **PR #12**: REFACTOR phase

**Files to Create/Modify**:

- `src/upload/put_object.rs` (modify)
- `src/upload/handler.rs` (new)
- `tests/upload_put_test.rs` (new)

**Acceptance Criteria**:

- [ ] Files up to 50MB upload successfully
- [ ] ETag matches uploaded content
- [ ] Content-Type preserved
- [ ] All tests pass: `cargo test --lib upload::put_object`

---

### 2.2 Multipart Upload Handler

**Goal**: Implement multipart upload for files >50MB

**TDD Workflow**:

1. 🔴 **RED**: Write failing tests for multipart upload

   - Test: Create multipart upload
   - Test: Upload parts
   - Test: Complete multipart upload
   - Test: Abort multipart upload
   - **PR #13**: RED phase

2. 🟢 **GREEN**: Implement multipart orchestration

   - Create `MultipartUploadManager`
   - Part upload with retry logic
   - Concurrent part uploads
   - **PR #14**: GREEN phase

3. 🔵 **REFACTOR**: Optimize part management
   - Use DashMap for concurrent part tracking
   - Implement part size calculation
   - **PR #15**: REFACTOR phase

**Files to Create/Modify**:

- `src/upload/multipart.rs` (modify)
- `src/upload/part_manager.rs` (new)
- `tests/upload_multipart_test.rs` (new)

**Acceptance Criteria**:

- [ ] Files >50MB trigger multipart upload
- [ ] Parts uploaded concurrently (configurable)
- [ ] Failed parts retry automatically
- [ ] All tests pass: `cargo test --lib upload::multipart`

---

### 2.3 Zero-Copy Integration

**Goal**: Integrate zero-copy transfer into upload handlers

**TDD Workflow**:

1. 🔴 **RED**: Write tests for zero-copy transfer

   - Test: Verify splice() called on Linux
   - Test: Fallback to buffered I/O on macOS
   - Test: Measure performance improvement
   - **PR #16**: RED phase

2. 🟢 **GREEN**: Integrate zero-copy into handlers

   - Use `DataTransfer` in PutObject
   - Use `DataTransfer` in multipart parts
   - **PR #17**: GREEN phase

3. 🔵 **REFACTOR**: Optimize transfer logic
   - Tune pipe buffer size
   - Add SPLICE_F_MORE flag
   - **PR #18**: REFACTOR phase

**Files to Create/Modify**:

- `src/upload/zero_copy.rs` (modify)
- `src/upload/put_object.rs` (modify)
- `benches/zero_copy_benchmark.rs` (modify)

**Acceptance Criteria**:

- [ ] Zero-copy used on Linux for uploads
- [ ] Buffered I/O used on other platforms
- [ ] Benchmarks show 50-250x speedup on Linux
- [ ] All tests pass: `cargo test --lib upload::zero_copy`

---

## Phase 3: Authentication (Security Layer)

### 3.1 JWT Authentication

**Goal**: Implement JWT token validation (HS256/RS256/ES256)

**Reference**: Reuse implementation from Yatagarasu project

**TDD Workflow**:

1. 🔴 **RED**: Write failing tests for JWT validation

   - Test: Validate HS256 token
   - Test: Validate RS256 token
   - Test: Reject expired token
   - Test: Reject invalid signature
   - **PR #19**: RED phase

2. 🟢 **GREEN**: Implement JWT validator

   - Use `jsonwebtoken` crate
   - Support multiple algorithms
   - Extract claims
   - **PR #20**: GREEN phase

3. 🔵 **REFACTOR**: Optimize token parsing
   - Cache public keys
   - Improve error messages
   - **PR #21**: REFACTOR phase

**Files to Create/Modify**:

- `src/auth/jwt.rs` (modify)
- `src/auth/validator.rs` (new)
- `tests/auth_jwt_test.rs` (new)

**Acceptance Criteria**:

- [ ] HS256, RS256, ES256 algorithms supported
- [ ] Token expiration checked
- [ ] Claims extracted correctly
- [ ] All tests pass: `cargo test --lib auth::jwt`

---

### 3.2 AWS SigV4 Authentication

**Goal**: Implement AWS Signature Version 4 validation

**TDD Workflow**:

1. 🔴 **RED**: Write failing tests for SigV4

   - Test: Validate SigV4 signature
   - Test: Reject tampered requests
   - Test: Handle query string auth
   - **PR #22**: RED phase

2. 🟢 **GREEN**: Implement SigV4 validator

   - Use `aws-sigv4` crate
   - Canonical request generation
   - Signature verification
   - **PR #23**: GREEN phase

3. 🔵 **REFACTOR**: Optimize signature validation
   - Cache credentials
   - Improve performance
   - **PR #24**: REFACTOR phase

**Files to Create/Modify**:

- `src/auth/sigv4.rs` (modify)
- `tests/auth_sigv4_test.rs` (new)

**Acceptance Criteria**:

- [ ] SigV4 signatures validated correctly
- [ ] Both header and query string auth work
- [ ] Timestamp validation prevents replay attacks
- [ ] All tests pass: `cargo test --lib auth::sigv4`

---

### 3.3 JWKS Support

**Goal**: Fetch and cache public keys from JWKS endpoints

**Reference**: Reuse implementation from Yatagarasu project

**TDD Workflow**:

1. 🔴 **RED**: Write failing tests for JWKS

   - Test: Fetch keys from JWKS URL
   - Test: Cache keys with TTL
   - Test: Refresh expired keys
   - **PR #25**: RED phase

2. 🟢 **GREEN**: Implement JWKS client

   - HTTP client for JWKS endpoint
   - Key caching with expiration
   - Background refresh
   - **PR #26**: GREEN phase

3. 🔵 **REFACTOR**: Optimize key management
   - Use parking_lot for faster locks
   - Implement key rotation
   - **PR #27**: REFACTOR phase

**Files to Create/Modify**:

- `src/auth/jwks.rs` (new)
- `src/auth/key_cache.rs` (new)
- `tests/auth_jwks_test.rs` (new)

**Acceptance Criteria**:

- [ ] JWKS keys fetched and cached
- [ ] Keys refresh automatically
- [ ] Multiple JWKS endpoints supported
- [ ] All tests pass: `cargo test --lib auth::jwks`

---

## Phase 4: Authorization (Access Control)

### 4.1 OPA Integration

**Goal**: Integrate Open Policy Agent for policy-based authorization

**Reference**: Reuse implementation from Yatagarasu project

**TDD Workflow**:

1. 🔴 **RED**: Write failing tests for OPA

   - Test: Allow upload with valid policy
   - Test: Deny upload with invalid policy
   - Test: Handle OPA server errors
   - **PR #28**: RED phase

2. 🟢 **GREEN**: Implement OPA client

   - HTTP client for OPA API
   - Policy evaluation
   - Input document generation
   - **PR #29**: GREEN phase

3. 🔵 **REFACTOR**: Optimize OPA calls
   - Connection pooling
   - Response caching
   - **PR #30**: REFACTOR phase

**Files to Create/Modify**:

- `src/authz/opa/mod.rs` (modify)
- `src/authz/opa/client.rs` (new)
- `tests/authz_opa_test.rs` (new)

**Acceptance Criteria**:

- [ ] OPA policies evaluated correctly
- [ ] Allow/deny decisions enforced
- [ ] OPA server failures handled gracefully
- [ ] All tests pass: `cargo test --lib authz::opa`

---

### 4.2 OpenFGA Integration

**Goal**: Integrate OpenFGA for fine-grained authorization

**Reference**: Reuse implementation from Yatagarasu project

**TDD Workflow**:

1. 🔴 **RED**: Write failing tests for OpenFGA

   - Test: Check user permissions
   - Test: Relationship-based access
   - Test: Handle OpenFGA errors
   - **PR #31**: RED phase

2. 🟢 **GREEN**: Implement OpenFGA client

   - gRPC client for OpenFGA
   - Check API integration
   - Tuple-based authorization
   - **PR #32**: GREEN phase

3. 🔵 **REFACTOR**: Optimize authorization checks
   - Batch permission checks
   - Cache authorization results
   - **PR #33**: REFACTOR phase

**Files to Create/Modify**:

- `src/authz/openfga/mod.rs` (modify)
- `src/authz/openfga/client.rs` (new)
- `tests/authz_openfga_test.rs` (new)

**Acceptance Criteria**:

- [ ] OpenFGA permissions checked correctly
- [ ] Relationship-based access works
- [ ] Errors handled gracefully
- [ ] All tests pass: `cargo test --lib authz::openfga`

---

## Phase 5: Production Ready (Polish & Performance)

### 5.1 Metrics HTTP Server

**Goal**: Expose Prometheus metrics on dedicated port

**TDD Workflow**:

1. 🔴 **RED**: Write failing tests for metrics endpoint

   - Test: Metrics endpoint returns Prometheus format
   - Test: Metrics updated on uploads
   - **PR #34**: RED phase

2. 🟢 **GREEN**: Implement metrics server

   - Separate HTTP server for metrics
   - `/metrics` endpoint
   - Prometheus text format
   - **PR #35**: GREEN phase

3. 🔵 **REFACTOR**: Optimize metrics collection
   - Reduce lock contention
   - Add more metrics
   - **PR #36**: REFACTOR phase

**Files to Create/Modify**:

- `src/metrics/server.rs` (new)
- `src/metrics/mod.rs` (modify)
- `tests/metrics_test.rs` (new)

**Acceptance Criteria**:

- [ ] Metrics server runs on configured port
- [ ] Prometheus can scrape metrics
- [ ] All upload metrics tracked
- [ ] All tests pass: `cargo test --lib metrics`

---

### 5.2 Error Handling & Logging

**Goal**: Comprehensive error handling and structured logging

**TDD Workflow**:

1. 🔴 **RED**: Write tests for error scenarios

   - Test: S3 errors mapped correctly
   - Test: Auth errors return 401
   - Test: AuthZ errors return 403
   - **PR #37**: RED phase

2. 🟢 **GREEN**: Implement error middleware

   - Error to HTTP status mapping
   - Structured error responses
   - Error logging
   - **PR #38**: GREEN phase

3. 🔵 **REFACTOR**: Improve error messages
   - User-friendly error messages
   - Debug information in logs
   - **PR #39**: REFACTOR phase

**Files to Create/Modify**:

- `src/server/error_handler.rs` (new)
- `src/server/middleware.rs` (new)
- `tests/error_handling_test.rs` (new)

**Acceptance Criteria**:

- [ ] All errors mapped to appropriate HTTP status
- [ ] Errors logged with context
- [ ] Error responses are JSON formatted
- [ ] All tests pass: `cargo test --lib server::error`

---

### 5.3 Performance Optimization & Benchmarks

**Goal**: Optimize performance and establish benchmarks

**TDD Workflow**:

1. 🔴 **RED**: Write benchmark tests

   - Benchmark: 1MB upload
   - Benchmark: 10MB upload
   - Benchmark: 100MB multipart upload
   - **PR #40**: RED phase

2. 🟢 **GREEN**: Optimize hot paths

   - Profile with `perf` on Linux
   - Optimize buffer sizes
   - Reduce allocations
   - **PR #41**: GREEN phase

3. 🔵 **REFACTOR**: Final optimizations
   - Tune Pingora settings
   - Optimize zero-copy parameters
   - **PR #42**: REFACTOR phase

**Files to Create/Modify**:

- `benches/upload_benchmark.rs` (modify)
- `benches/zero_copy_benchmark.rs` (modify)
- `benches/end_to_end_benchmark.rs` (new)

**Acceptance Criteria**:

- [ ] Benchmarks show expected performance
- [ ] Zero-copy provides 50-250x speedup on Linux
- [ ] No performance regressions
- [ ] All benchmarks pass: `cargo bench`

---

## Testing Strategy

### Unit Tests

- **Location**: Inline in `src/**/*.rs` files
- **Command**: `cargo test --lib`
- **Coverage**: Each module has `#[cfg(test)] mod tests`

### Integration Tests

- **Location**: `tests/` directory
- **Command**: `cargo test --test '*'`
- **Coverage**: Component interactions, end-to-end flows

### Benchmarks

- **Location**: `benches/` directory
- **Command**: `cargo bench`
- **Coverage**: Performance regression detection

### E2E Tests

- **Location**: Python scripts (e.g., `test_zero_copy.py`)
- **Command**: `python3 test_*.py`
- **Coverage**: Full system validation with real S3

---

## Quality Gates (Before Each PR Merge)

Every PR MUST pass these checks:

1. ✅ **All Tests Pass**: `cargo test --all-features`
2. ✅ **No Clippy Warnings**: `cargo clippy -- -D warnings`
3. ✅ **Code Formatted**: `cargo fmt --check`
4. ✅ **Documentation**: `cargo doc --no-deps`
5. ✅ **No Security Issues**: `cargo audit` (if available)

---

## Estimated Timeline

| Phase                        | PRs        | Estimated Time  |
| ---------------------------- | ---------- | --------------- |
| Phase 1: Core Infrastructure | PR #1-9    | 2-3 weeks       |
| Phase 2: Upload Operations   | PR #10-18  | 3-4 weeks       |
| Phase 3: Authentication      | PR #19-27  | 2-3 weeks       |
| Phase 4: Authorization       | PR #28-33  | 2 weeks         |
| Phase 5: Production Ready    | PR #34-42  | 2-3 weeks       |
| **Total**                    | **42 PRs** | **11-15 weeks** |

---

## Dependencies & References

### External Crates to Add

- `pingora` - HTTP proxy framework
- `pingora-core` - Core Pingora functionality
- `tonic` - gRPC client for OpenFGA (if needed)

### Reference Projects

- **Yatagarasu**: JWT, OPA, OpenFGA, metrics implementations
- **Pingora Examples**: Server setup patterns

### Documentation

- [Pingora Documentation](https://github.com/cloudflare/pingora)
- [AWS S3 API Reference](https://docs.aws.amazon.com/AmazonS3/latest/API/)
- [Linux splice(2) man page](https://man7.org/linux/man-pages/man2/splice.2.html)

---

## Success Criteria

The implementation is complete when:

1. ✅ All 42 PRs merged
2. ✅ All tests passing (unit, integration, E2E)
3. ✅ Benchmarks show expected performance
4. ✅ Docker image builds successfully
5. ✅ Documentation complete
6. ✅ Can upload files to S3 via the proxy
7. ✅ Authentication and authorization work
8. ✅ Metrics exposed and scrapable
9. ✅ Zero-copy provides measured speedup on Linux
10. ✅ Production deployment successful

---

## Next Steps

1. **Review this plan** with the team
2. **Set up CI/CD pipeline** for automated testing
3. **Create GitHub project board** to track PRs
4. **Start Phase 1.1** - HTTP Server with Pingora
5. **Follow TDD strictly** - Red, Green, Refactor for each feature

---

_Last Updated: 2025-12-25_
_Methodology: TDD Red-Green-Refactor with PR-per-Phase_
