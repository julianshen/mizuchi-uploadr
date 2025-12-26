# Distributed Tracing with OpenTelemetry

Mizuchi Uploadr provides comprehensive distributed tracing support using OpenTelemetry, enabling you to monitor and debug your S3 upload operations across distributed systems.

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [Supported Backends](#supported-backends)
- [Instrumentation](#instrumentation)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## Overview

### What is Distributed Tracing?

Distributed tracing tracks requests as they flow through your system, creating a timeline of operations (spans) that show:

- How long each operation took
- Which operations happened in parallel vs. sequentially
- Where errors occurred
- How requests propagate across service boundaries

### Why Use Tracing in Mizuchi Uploadr?

- **Upload Performance**: See exactly where time is spent during uploads (auth, S3 operations, zero-copy transfers)
- **Error Diagnosis**: Quickly identify which component failed (authentication, authorization, S3 client)
- **Security Auditing**: Track authentication and authorization decisions
- **Capacity Planning**: Understand upload patterns and resource usage

## Quick Start

### 1. Enable Tracing Feature

Add the `tracing` feature to your `Cargo.toml`:

```toml
[dependencies]
mizuchi-uploadr = { version = "0.1", features = ["tracing"] }
```

### 2. Configure Tracing

Create a `config.yaml` with tracing configuration:

```yaml
tracing:
  enabled: true
  service_name: "mizuchi-uploadr"

  otlp:
    endpoint: "http://localhost:4317" # Jaeger/Tempo endpoint
    protocol: "grpc"
    timeout_seconds: 10

  sampling:
    strategy: "always" # Sample all traces in development
    ratio: 1.0

  batch:
    max_queue_size: 2048
    scheduled_delay_millis: 5000
    max_export_batch_size: 512
```

### 3. Initialize Tracing

```rust
use mizuchi_uploadr::config::ConfigLoader;
use mizuchi_uploadr::tracing::init::init_tracing;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let config = ConfigLoader::from_file("config.yaml")?;

    // Initialize tracing (returns a guard that flushes on drop)
    let _tracing_guard = init_tracing(&config.tracing)?;

    // Your application code here
    // Traces will be automatically exported

    Ok(())
    // Guard drops here, flushing all pending spans
}
```

### 4. Run a Tracing Backend

**Option A: Jaeger (Recommended for Development)**

```bash
docker run -d --name jaeger \
  -p 4317:4317 \
  -p 16686:16686 \
  jaegertracing/all-in-one:latest
```

Access Jaeger UI at http://localhost:16686

**Option B: Grafana Tempo**

```bash
docker run -d --name tempo \
  -p 4317:4317 \
  grafana/tempo:latest \
  -config.file=/etc/tempo.yaml
```

See [Supported Backends](#supported-backends) for more options.

## Configuration

### Tracing Configuration Reference

| Field          | Type   | Default             | Description            |
| -------------- | ------ | ------------------- | ---------------------- |
| `enabled`      | bool   | `false`             | Enable/disable tracing |
| `service_name` | string | `"mizuchi-uploadr"` | Service name in traces |

### OTLP Configuration

| Field             | Type    | Default  | Description                                   |
| ----------------- | ------- | -------- | --------------------------------------------- |
| `endpoint`        | string  | `""`     | OTLP collector endpoint (required if enabled) |
| `protocol`        | string  | `"grpc"` | Protocol: `grpc` or `http/protobuf`           |
| `timeout_seconds` | u64     | `10`     | Export timeout in seconds                     |
| `compression`     | string? | `null`   | Compression: `gzip` or `none`                 |

### Sampling Configuration

| Field      | Type   | Default    | Description                                   |
| ---------- | ------ | ---------- | --------------------------------------------- |
| `strategy` | string | `"always"` | Sampling strategy (see below)                 |
| `ratio`    | f64    | `1.0`      | Sampling ratio (0.0-1.0) for `ratio` strategy |

**Sampling Strategies:**

- `always` - Sample all traces (100%, recommended for development)
- `never` - Sample no traces (0%, useful for disabling)
- `ratio` - Sample a percentage based on `ratio` field
- `parent_based` - Respect parent span's sampling decision

### Batch Configuration

| Field                    | Type  | Default | Description                              |
| ------------------------ | ----- | ------- | ---------------------------------------- |
| `max_queue_size`         | usize | `2048`  | Max spans to queue before forcing export |
| `scheduled_delay_millis` | u64   | `5000`  | Delay between scheduled exports (ms)     |
| `max_export_batch_size`  | usize | `512`   | Max spans per export batch               |

### Environment Variable Expansion

Configuration values support environment variable expansion:

```yaml
tracing:
  service_name: "${SERVICE_NAME:-mizuchi-uploadr}" # With default
  otlp:
    endpoint: "${OTLP_ENDPOINT}" # Required env var
```

## Supported Backends

Mizuchi Uploadr supports any OTLP-compatible tracing backend:

### Jaeger

**Best for:** Development, quick setup

```yaml
otlp:
  endpoint: "http://localhost:4317"
  protocol: "grpc"
```

**Docker:**

```bash
docker run -d --name jaeger \
  -p 4317:4317 \
  -p 16686:16686 \
  jaegertracing/all-in-one:latest
```

**UI:** http://localhost:16686

### Grafana Tempo

**Best for:** Production, long-term storage, integration with Grafana

```yaml
otlp:
  endpoint: "http://tempo:4317"
  protocol: "grpc"
```

**Docker Compose:**

```yaml
services:
  tempo:
    image: grafana/tempo:latest
    ports:
      - "4317:4317"
    volumes:
      - ./tempo.yaml:/etc/tempo.yaml
```

### Honeycomb

**Best for:** Production, advanced analysis, SaaS

```yaml
otlp:
  endpoint: "https://api.honeycomb.io:443"
  protocol: "grpc"
```

Set API key via environment variable:

```bash
export HONEYCOMB_API_KEY="your-api-key"
```

### New Relic

**Best for:** Enterprise, APM integration

```yaml
otlp:
  endpoint: "https://otlp.nr-data.net:4317"
  protocol: "grpc"
```

### Datadog

**Best for:** Enterprise, full observability stack

```yaml
otlp:
  endpoint: "http://localhost:4318"
  protocol: "http/protobuf"
```

## Instrumentation

Mizuchi Uploadr automatically instruments the following operations:

### HTTP Requests

Every HTTP request creates a root span with:

- **Span Name**: `http.request`
- **Attributes**:
  - `http.method` - HTTP method (PUT, POST, DELETE)
  - `http.route` - Request path pattern
  - `http.status_code` - Response status code
  - `http.request_content_length` - Request body size
  - `otel.kind` - `server`

### Authentication

Authentication operations create spans with:

- **Span Name**: `auth.jwt` or `auth.sigv4`
- **Attributes**:
  - `auth.method` - Authentication method (jwt, sigv4)
  - `auth.token_present` - Whether token/signature was provided
  - `subject` - Authenticated user (JWT) or access key (SigV4)
  - `otel.kind` - `internal`

**Security Note**: Actual tokens and secrets are NEVER included in spans.

### Authorization

Authorization checks create spans with:

- **Span Name**: `authz.opa` or `authz.openfga`
- **Attributes**:
  - `authz.method` - Authorization method (opa, openfga)
  - `authz.action` - Action being authorized (upload, delete)
  - `authz.resource_type` - Resource type (bucket, object) - NOT full path
  - `authz.relation` - OpenFGA relation (for OpenFGA only)
  - `decision` - Authorization decision (allow, deny)
  - `otel.kind` - `internal`

**Security Note**: User identities and full resource paths are NOT included to prevent PII leakage.

### Upload Operations

Upload operations create spans with:

- **Span Name**: `upload.put_object` or `upload.multipart`
- **Attributes**:
  - `upload.type` - Upload type (simple, multipart)
  - `upload.size` - Upload size in bytes
  - `upload.bucket` - Target bucket name
  - `upload.zero_copy` - Whether zero-copy was used (Linux only)
  - `otel.kind` - `internal`

### S3 Operations

S3 client operations create spans with:

- **Span Name**: `s3.put_object`, `s3.create_multipart_upload`, etc.
- **Attributes**:
  - `s3.operation` - S3 operation name
  - `s3.bucket` - Bucket name
  - `s3.region` - AWS region
  - `otel.kind` - `client`

### Context Propagation

Mizuchi Uploadr supports W3C Trace Context propagation:

- Incoming requests: Extracts `traceparent` and `tracestate` headers
- Outgoing requests: Injects trace context into S3 API calls
- Enables end-to-end tracing across services

## Best Practices

### Development

```yaml
tracing:
  enabled: true
  sampling:
    strategy: "always" # Sample all traces
    ratio: 1.0
  batch:
    scheduled_delay_millis: 1000 # Export frequently for quick feedback
```

### Production

```yaml
tracing:
  enabled: true
  sampling:
    strategy: "ratio" # Sample a percentage
    ratio: 0.1 # 10% of traces
  batch:
    max_queue_size: 4096 # Larger queue for high throughput
    scheduled_delay_millis: 5000 # Less frequent exports
    max_export_batch_size: 1024
```

### High-Traffic Production

```yaml
tracing:
  enabled: true
  sampling:
    strategy: "parent_based" # Respect upstream sampling decisions
    ratio: 0.01 # 1% of traces
  batch:
    max_queue_size: 8192
    scheduled_delay_millis: 10000
    max_export_batch_size: 2048
  otlp:
    compression: "gzip" # Reduce network bandwidth
```

### Security Considerations

1. **No PII in Spans**: Mizuchi Uploadr never includes:

   - JWT tokens or SigV4 signatures
   - User emails or personal information
   - Full S3 object keys (only resource types)
   - File contents

2. **Secure Endpoints**: Always use HTTPS for production OTLP endpoints:

   ```yaml
   otlp:
     endpoint: "https://otlp.example.com:4317"
   ```

3. **Authentication**: Configure OTLP endpoint authentication via environment variables:
   ```bash
   export OTEL_EXPORTER_OTLP_HEADERS="api-key=your-key"
   ```

### Performance Tuning

1. **Sampling**: Reduce overhead in high-traffic scenarios:

   ```yaml
   sampling:
     strategy: "ratio"
     ratio: 0.05 # 5% sampling
   ```

2. **Batch Size**: Tune for your workload:

   - Smaller batches = lower latency, more network calls
   - Larger batches = higher latency, fewer network calls

3. **Queue Size**: Prevent memory issues:
   ```yaml
   batch:
     max_queue_size: 2048 # Adjust based on available memory
   ```

## Troubleshooting

### Traces Not Appearing

**Check 1: Is tracing enabled?**

```yaml
tracing:
  enabled: true # Must be true
```

**Check 2: Is the OTLP endpoint correct?**

```bash
# Test connectivity
curl http://localhost:4317
```

**Check 3: Are spans being sampled?**

```yaml
sampling:
  strategy: "always" # Try always-on for debugging
```

**Check 4: Check application logs**

```bash
# Look for tracing initialization messages
RUST_LOG=debug cargo run
```

### High Memory Usage

**Symptom**: Application memory grows over time

**Solution**: Reduce queue size and batch size:

```yaml
batch:
  max_queue_size: 1024 # Reduce from default 2048
  max_export_batch_size: 256 # Reduce from default 512
```

### Slow Performance

**Symptom**: Application is slower with tracing enabled

**Solution 1**: Reduce sampling:

```yaml
sampling:
  strategy: "ratio"
  ratio: 0.1 # Sample only 10%
```

**Solution 2**: Increase export delay:

```yaml
batch:
  scheduled_delay_millis: 10000 # Export less frequently
```

**Solution 3**: Use compression:

```yaml
otlp:
  compression: "gzip"
```

### Connection Timeouts

**Symptom**: "Failed to export spans: timeout"

**Solution**: Increase timeout:

```yaml
otlp:
  timeout_seconds: 30 # Increase from default 10
```

### Missing Spans

**Symptom**: Some spans don't appear in traces

**Possible Causes**:

1. **Sampling**: Spans may be sampled out
2. **Queue Overflow**: Queue may be full, causing drops
3. **Export Failure**: Backend may be rejecting spans

**Solution**: Enable debug logging:

```bash
RUST_LOG=opentelemetry=debug cargo run
```

## Examples

See the `examples/` directory for complete working examples:

- `examples/tracing_jaeger.rs` - Jaeger integration
- `examples/tracing_tempo.rs` - Grafana Tempo integration
- `examples/tracing_custom.rs` - Custom instrumentation

## Further Reading

- [OpenTelemetry Documentation](https://opentelemetry.io/docs/)
- [W3C Trace Context Specification](https://www.w3.org/TR/trace-context/)
- [Jaeger Documentation](https://www.jaegertracing.io/docs/)
- [Grafana Tempo Documentation](https://grafana.com/docs/tempo/)
