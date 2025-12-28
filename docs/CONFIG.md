# Configuration Reference

Complete configuration reference for Mizuchi Uploadr.

## Table of Contents

- [Overview](#overview)
- [Server Configuration](#server-configuration)
- [Bucket Configuration](#bucket-configuration)
- [Authentication Configuration](#authentication-configuration)
- [Authorization Configuration](#authorization-configuration)
- [Upload Configuration](#upload-configuration)
- [Metrics Configuration](#metrics-configuration)
- [Tracing Configuration](#tracing-configuration)
- [Environment Variables](#environment-variables)
- [Complete Examples](#complete-examples)

---

## Overview

Mizuchi Uploadr uses YAML configuration files. By default, it looks for `config.yaml` in the current directory.

```bash
# Specify config file
mizuchi-uploadr --config /path/to/config.yaml

# Use environment variable
MIZUCHI_CONFIG=/path/to/config.yaml mizuchi-uploadr
```

### Configuration Structure

```yaml
server:          # Server settings
  address: "..."
  zero_copy: { ... }

buckets:         # List of bucket configurations
  - name: "..."
    path_prefix: "..."
    s3: { ... }
    auth: { ... }
    authz: { ... }
    upload: { ... }

metrics:         # Prometheus metrics
  enabled: true
  port: 9090

tracing:         # OpenTelemetry tracing
  enabled: true
  otlp: { ... }
```

---

## Server Configuration

### Basic Settings

```yaml
server:
  address: "0.0.0.0:8080"   # Listen address (host:port)
  zero_copy:
    enabled: true           # Enable zero-copy on Linux
    pipe_buffer_size: 1048576  # Pipe buffer size (default 1MB)
```

### Configuration Options

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `address` | string | `"0.0.0.0:8080"` | Server listen address |
| `zero_copy.enabled` | bool | `true` | Enable Linux zero-copy (splice/sendfile) |
| `zero_copy.pipe_buffer_size` | number | `1048576` | Pipe buffer size in bytes |

### Zero-Copy Notes

- **Linux only**: Zero-copy uses `splice(2)` and `sendfile(2)` syscalls
- **Fallback**: On macOS/Windows, falls back to buffered I/O
- **Performance**: 50-250x speedup for large files on Linux

---

## Bucket Configuration

Each bucket maps a URL path prefix to an S3 backend.

```yaml
buckets:
  - name: "uploads"           # Bucket identifier (for logs/metrics)
    path_prefix: "/uploads"   # URL path prefix
    s3:
      bucket: "my-s3-bucket"  # S3 bucket name
      region: "us-east-1"     # AWS region
      endpoint: "..."         # Optional: Custom endpoint (MinIO, etc.)
      access_key: "..."       # AWS access key
      secret_key: "..."       # AWS secret key
    auth: { ... }             # Authentication config
    authz: { ... }            # Authorization config
    upload: { ... }           # Upload behavior config
```

### Path Matching

Paths are matched by prefix with boundary detection:

| Request Path | Bucket Prefix | Match? |
|--------------|---------------|--------|
| `/uploads/file.txt` | `/uploads` | Yes |
| `/uploads/dir/file.txt` | `/uploads` | Yes |
| `/uploads2/file.txt` | `/uploads` | **No** (boundary check) |
| `/uploads` | `/uploads` | Yes (exact match) |

When multiple buckets match, the longest prefix wins.

### S3 Backend Configuration

```yaml
s3:
  bucket: "my-bucket"                    # Required: S3 bucket name
  region: "us-east-1"                    # Required: AWS region
  endpoint: "http://localhost:9000"      # Optional: Custom endpoint
  access_key: "${AWS_ACCESS_KEY_ID}"     # Required: Access key
  secret_key: "${AWS_SECRET_ACCESS_KEY}" # Required: Secret key
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `bucket` | string | - | S3 bucket name (required) |
| `region` | string | - | AWS region (required) |
| `endpoint` | string | AWS default | Custom S3 endpoint URL |
| `access_key` | string | - | AWS access key ID |
| `secret_key` | string | - | AWS secret access key |

### S3-Compatible Services

**MinIO:**
```yaml
s3:
  bucket: "my-bucket"
  region: "us-east-1"
  endpoint: "http://localhost:9000"
  access_key: "minioadmin"
  secret_key: "minioadmin"
```

**Cloudflare R2:**
```yaml
s3:
  bucket: "my-bucket"
  region: "auto"
  endpoint: "https://<account-id>.r2.cloudflarestorage.com"
  access_key: "${R2_ACCESS_KEY}"
  secret_key: "${R2_SECRET_KEY}"
```

**DigitalOcean Spaces:**
```yaml
s3:
  bucket: "my-bucket"
  region: "nyc3"
  endpoint: "https://nyc3.digitaloceanspaces.com"
  access_key: "${DO_ACCESS_KEY}"
  secret_key: "${DO_SECRET_KEY}"
```

---

## Authentication Configuration

### Disable Authentication

```yaml
auth:
  enabled: false  # No authentication required
```

### JWT Authentication (HS256)

```yaml
auth:
  enabled: true
  jwt:
    algorithm: "HS256"
    secret: "${JWT_SECRET}"             # Shared secret
    issuer: "https://auth.example.com"  # Optional: Validate issuer
    audience: "mizuchi-uploadr"         # Optional: Validate audience
```

### JWT Authentication (RS256/ES256)

```yaml
auth:
  enabled: true
  jwt:
    algorithm: "RS256"                  # or "ES256"
    public_key: |
      -----BEGIN PUBLIC KEY-----
      MIIBIjANBgkqhkiG9...
      -----END PUBLIC KEY-----
    issuer: "https://auth.example.com"
    audience: "mizuchi-uploadr"
```

### JWT with JWKS Endpoint

```yaml
auth:
  enabled: true
  jwt:
    algorithm: "RS256"
    jwks_url: "https://auth.example.com/.well-known/jwks.json"
    cache_ttl_seconds: 3600  # Cache JWKS for 1 hour
    issuer: "https://auth.example.com"
    audience: "mizuchi-uploadr"
```

### AWS SigV4 Authentication

```yaml
auth:
  enabled: true
  sigv4:
    access_key: "${SIGV4_ACCESS_KEY}"
    secret_key: "${SIGV4_SECRET_KEY}"
    region: "us-east-1"
    max_clock_skew_seconds: 300  # Optional: Default 5 minutes
```

### Token Sources

Configure where to look for JWT tokens:

```yaml
auth:
  enabled: true
  jwt:
    secret: "${JWT_SECRET}"
    token_sources:
      - type: "bearer"           # Authorization: Bearer <token>
      - type: "query"
        name: "token"            # ?token=<token>
      - type: "header"
        name: "X-Auth-Token"     # X-Auth-Token: <token>
```

### JWT Configuration Options

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `algorithm` | string | `"HS256"` | JWT algorithm (HS256, RS256, ES256) |
| `secret` | string | - | Shared secret (HS256 only) |
| `public_key` | string | - | Public key PEM (RS256/ES256) |
| `jwks_url` | string | - | JWKS endpoint URL |
| `cache_ttl_seconds` | number | `3600` | JWKS cache duration |
| `issuer` | string | - | Required issuer claim |
| `audience` | string | - | Required audience claim |
| `token_sources` | list | Bearer | Where to find tokens |

---

## Authorization Configuration

### Disable Authorization

```yaml
authz:
  enabled: false  # No authorization checks
```

### OPA (Open Policy Agent)

```yaml
authz:
  enabled: true
  opa:
    url: "http://localhost:8181"
    policy_path: "/v1/data/mizuchi/allow"
    timeout_seconds: 5
    cache_ttl_seconds: 60
    cache_max_entries: 1000
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | string | - | OPA server URL (required) |
| `policy_path` | string | - | Policy evaluation path (required) |
| `timeout_seconds` | number | `5` | Request timeout |
| `cache_ttl_seconds` | number | `60` | Decision cache TTL |
| `cache_max_entries` | number | `1000` | Max cache entries |

### OpenFGA

```yaml
authz:
  enabled: true
  openfga:
    url: "http://localhost:8080"
    store_id: "${OPENFGA_STORE_ID}"
    model_id: "${OPENFGA_MODEL_ID}"  # Optional
    timeout_seconds: 5
    cache_ttl_seconds: 60
    cache_max_entries: 1000
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `url` | string | - | OpenFGA server URL (required) |
| `store_id` | string | - | Store ID (required) |
| `model_id` | string | - | Authorization model ID |
| `timeout_seconds` | number | `5` | Request timeout |
| `cache_ttl_seconds` | number | `60` | Decision cache TTL |
| `cache_max_entries` | number | `1000` | Max cache entries |

---

## Upload Configuration

```yaml
upload:
  multipart_threshold: 52428800  # 50MB - Use multipart above this size
  part_size: 104857600           # 100MB - Size of each part
  concurrent_parts: 4            # Parallel part uploads
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `multipart_threshold` | number | `52428800` | Use multipart above this size (bytes) |
| `part_size` | number | `104857600` | Size of each multipart chunk |
| `concurrent_parts` | number | `4` | Parallel part uploads |

### Upload Size Recommendations

| File Size | Recommendation |
|-----------|----------------|
| < 50 MB | Single PUT (default) |
| 50 MB - 5 GB | Multipart (automatic) |
| > 5 GB | Must use multipart |

---

## Metrics Configuration

```yaml
metrics:
  enabled: true   # Enable Prometheus metrics
  port: 9090      # Metrics HTTP server port
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enabled` | bool | `false` | Enable metrics server |
| `port` | number | `9090` | Metrics HTTP port |

Access metrics at `http://localhost:9090/metrics`

---

## Tracing Configuration

See [TRACING.md](TRACING.md) for complete tracing documentation.

```yaml
tracing:
  enabled: true
  service_name: "mizuchi-uploadr"

  otlp:
    endpoint: "http://localhost:4317"
    protocol: "grpc"
    timeout_seconds: 10
    compression: "gzip"

  sampling:
    strategy: "always"   # always, never, ratio, parent_based
    ratio: 1.0

  batch:
    max_queue_size: 2048
    scheduled_delay_millis: 5000
    max_export_batch_size: 512
```

---

## Environment Variables

All configuration values support environment variable expansion:

```yaml
# Direct substitution
secret: "${JWT_SECRET}"

# With default value
service_name: "${SERVICE_NAME:-mizuchi-uploadr}"
```

### Expansion Syntax

| Syntax | Description |
|--------|-------------|
| `${VAR}` | Substitute with $VAR value |
| `${VAR:-default}` | Use default if VAR is unset or empty |
| `${VAR-default}` | Use default if VAR is unset |

### Common Environment Variables

| Variable | Description |
|----------|-------------|
| `AWS_ACCESS_KEY_ID` | AWS access key |
| `AWS_SECRET_ACCESS_KEY` | AWS secret key |
| `JWT_SECRET` | JWT signing secret |
| `OTLP_ENDPOINT` | OpenTelemetry collector endpoint |
| `SERVICE_NAME` | Service name for tracing |

---

## Complete Examples

### Development (MinIO + No Auth)

```yaml
server:
  address: "0.0.0.0:8080"
  zero_copy:
    enabled: false  # macOS development

buckets:
  - name: "dev-uploads"
    path_prefix: "/uploads"
    s3:
      bucket: "dev-bucket"
      region: "us-east-1"
      endpoint: "http://localhost:9000"
      access_key: "minioadmin"
      secret_key: "minioadmin"
    auth:
      enabled: false
    upload:
      multipart_threshold: 10485760  # 10MB for testing

metrics:
  enabled: true
  port: 9090

tracing:
  enabled: true
  service_name: "mizuchi-uploadr-dev"
  otlp:
    endpoint: "http://localhost:4317"
  sampling:
    strategy: "always"
```

### Production (AWS + JWT + OPA)

```yaml
server:
  address: "0.0.0.0:8080"
  zero_copy:
    enabled: true
    pipe_buffer_size: 2097152  # 2MB

buckets:
  - name: "production-uploads"
    path_prefix: "/uploads"
    s3:
      bucket: "${S3_BUCKET}"
      region: "${AWS_REGION}"
      access_key: "${AWS_ACCESS_KEY_ID}"
      secret_key: "${AWS_SECRET_ACCESS_KEY}"
    auth:
      enabled: true
      jwt:
        algorithm: "RS256"
        jwks_url: "https://auth.example.com/.well-known/jwks.json"
        issuer: "https://auth.example.com"
        audience: "mizuchi-uploadr"
    authz:
      enabled: true
      opa:
        url: "http://opa:8181"
        policy_path: "/v1/data/mizuchi/allow"
    upload:
      multipart_threshold: 52428800
      part_size: 104857600
      concurrent_parts: 8

metrics:
  enabled: true
  port: 9090

tracing:
  enabled: true
  service_name: "mizuchi-uploadr-prod"
  otlp:
    endpoint: "https://otlp.example.com:4317"
    compression: "gzip"
  sampling:
    strategy: "ratio"
    ratio: 0.1  # 10% sampling
  batch:
    max_queue_size: 4096
    max_export_batch_size: 1024
```

### Multi-Bucket Setup

```yaml
server:
  address: "0.0.0.0:8080"

buckets:
  # Public uploads - no auth
  - name: "public"
    path_prefix: "/public"
    s3:
      bucket: "public-uploads"
      region: "us-east-1"
    auth:
      enabled: false

  # User uploads - JWT auth
  - name: "user-uploads"
    path_prefix: "/users"
    s3:
      bucket: "user-uploads"
      region: "us-east-1"
    auth:
      enabled: true
      jwt:
        secret: "${JWT_SECRET}"

  # Admin uploads - JWT + OPA
  - name: "admin"
    path_prefix: "/admin"
    s3:
      bucket: "admin-uploads"
      region: "us-east-1"
    auth:
      enabled: true
      jwt:
        secret: "${JWT_SECRET}"
    authz:
      enabled: true
      opa:
        url: "http://opa:8181"
        policy_path: "/v1/data/admin/allow"
```

---

## Configuration Validation

Run with `--validate` to check configuration without starting:

```bash
mizuchi-uploadr --config config.yaml --validate
```

Common validation errors:

| Error | Cause |
|-------|-------|
| "Invalid address" | Malformed server address |
| "Empty buckets" | No buckets configured |
| "Missing bucket" | S3 bucket name not set |
| "Missing region" | AWS region not set |
| "JWT enabled but no secret" | Auth misconfiguration |

---

## Further Reading

- [API Reference](API.md)
- [Tracing Guide](TRACING.md)
- [Deployment Guide](DEPLOYMENT.md)
