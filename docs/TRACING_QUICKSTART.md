# OpenTelemetry Tracing - Quick Start Guide

> Quick reference for implementing and using distributed tracing in Mizuchi Uploadr

## 🚀 Quick Setup

### 1. Enable Tracing Feature

```bash
# Build with tracing support
cargo build --features tracing

# Run with tracing enabled
cargo run --features tracing -- --config config.yaml
```

### 2. Configure Tracing Backend

**Start Jaeger (for local development)**:
```bash
docker run -d \
  --name jaeger \
  -p 4317:4317 \
  -p 16686:16686 \
  jaegertracing/all-in-one:latest
```

**Access Jaeger UI**: http://localhost:16686

### 3. Update Configuration

**config.yaml**:
```yaml
tracing:
  enabled: true
  service_name: "mizuchi-uploadr"
  
  otlp:
    endpoint: "http://localhost:4317"
    protocol: "grpc"
    timeout_seconds: 10
  
  sampling:
    strategy: "always"  # Sample all traces in dev
    ratio: 1.0
```

### 4. Test Tracing

```bash
# Upload a file
curl -X PUT http://localhost:8080/uploads/test.txt \
  -H "Content-Type: text/plain" \
  -d "Hello, tracing!"

# View trace in Jaeger UI
open http://localhost:16686
```

---

## 📝 Adding Tracing to Your Code

### Instrument a Function

```rust
use tracing::{instrument, info, error};

#[instrument(
    name = "upload.put_object",
    skip(body),  // Don't log large data
    fields(
        bucket = %bucket,
        key = %key,
        size = body.len()
    )
)]
async fn upload_file(
    bucket: &str,
    key: &str,
    body: Bytes,
) -> Result<UploadResult, UploadError> {
    info!("Starting upload");
    
    // Your upload logic here
    let result = s3_client.put_object(bucket, key, body).await?;
    
    info!(etag = %result.etag, "Upload successful");
    Ok(result)
}
```

### Create Manual Spans

```rust
use tracing::{span, Level};

async fn process_multipart_upload() -> Result<(), Error> {
    let span = span!(Level::INFO, "multipart.upload", parts = 10);
    let _enter = span.enter();
    
    // Your multipart logic here
    
    Ok(())
}
```

### Add Span Attributes

```rust
use tracing::Span;

let current_span = Span::current();
current_span.record("upload.method", "multipart");
current_span.record("upload.parts", 10);
current_span.record("upload.zero_copy", true);
```

### Log Events in Spans

```rust
use tracing::{info, warn, error};

info!(bytes_transferred = 1024, "Chunk uploaded");
warn!(retry_count = 3, "Retrying upload");
error!(error = %e, "Upload failed");
```

---

## 🔍 Trace Context Propagation

### Extract Context from HTTP Headers

```rust
use opentelemetry::global;
use opentelemetry::propagation::Extractor;

struct HeaderExtractor<'a>(&'a hyper::HeaderMap);

impl<'a> Extractor for HeaderExtractor<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| v.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_str()).collect()
    }
}

// In your HTTP handler
let parent_cx = global::get_text_map_propagator(|propagator| {
    propagator.extract(&HeaderExtractor(&req.headers()))
});
```

### Inject Context into Outgoing Requests

```rust
use opentelemetry::global;
use opentelemetry::propagation::Injector;

struct HeaderInjector<'a>(&'a mut hyper::HeaderMap);

impl<'a> Injector for HeaderInjector<'a> {
    fn set(&mut self, key: &str, value: String) {
        if let Ok(name) = hyper::header::HeaderName::from_bytes(key.as_bytes()) {
            if let Ok(val) = hyper::header::HeaderValue::from_str(&value) {
                self.0.insert(name, val);
            }
        }
    }
}

// Before making S3 request
let mut headers = hyper::HeaderMap::new();
global::get_text_map_propagator(|propagator| {
    propagator.inject_context(&cx, &mut HeaderInjector(&mut headers))
});
```

---

## 📊 Span Attributes Reference

### HTTP Spans

```rust
span.set_attribute("http.method", "PUT");
span.set_attribute("http.target", "/uploads/file.txt");
span.set_attribute("http.status_code", 200);
span.set_attribute("http.request_content_length", 1024);
span.set_attribute("http.response_content_length", 256);
span.set_attribute("net.peer.ip", "192.168.1.1");
span.set_attribute("net.peer.port", 54321);
```

### S3 Upload Spans

```rust
span.set_attribute("s3.bucket", "my-bucket");
span.set_attribute("s3.key", "path/to/file.txt");
span.set_attribute("s3.operation", "PutObject");
span.set_attribute("s3.region", "us-east-1");
span.set_attribute("upload.size_bytes", 10485760);
span.set_attribute("upload.method", "multipart");
span.set_attribute("upload.parts", 10);
span.set_attribute("upload.zero_copy", true);
```

### Authentication Spans

```rust
span.set_attribute("auth.method", "jwt");
span.set_attribute("auth.algorithm", "RS256");
span.set_attribute("auth.success", true);
// DO NOT log sensitive data like tokens or passwords!
```

---

## 🎯 Sampling Strategies

### Always Sample (Development)

```yaml
tracing:
  sampling:
    strategy: "always"
```

### Ratio-Based Sampling (Production)

```yaml
tracing:
  sampling:
    strategy: "ratio"
    ratio: 0.1  # Sample 10% of traces
```

### Parent-Based Sampling (Distributed)

```yaml
tracing:
  sampling:
    strategy: "parent_based"
    # Respects upstream sampling decision
```

---

## 🐛 Troubleshooting

### Traces Not Appearing in Jaeger

1. **Check OTLP endpoint**:
   ```bash
   curl http://localhost:4317
   ```

2. **Verify feature flag**:
   ```bash
   cargo build --features tracing
   ```

3. **Check logs for export errors**:
   ```bash
   RUST_LOG=opentelemetry=debug cargo run --features tracing
   ```

### High Tracing Overhead

1. **Reduce sampling rate**:
   ```yaml
   sampling:
     ratio: 0.01  # Sample 1%
   ```

2. **Optimize batch settings**:
   ```yaml
   batch:
     max_queue_size: 4096
     scheduled_delay_millis: 10000
   ```

3. **Disable tracing for hot paths**:
   ```rust
   #[instrument(skip_all)]  // Skip all arguments
   ```

### Missing Span Attributes

1. **Check attribute limits** (OpenTelemetry has defaults)
2. **Verify attribute types** (must be string, int, float, or bool)
3. **Check for errors in logs**

---

## 📚 Additional Resources

- [OpenTelemetry Rust Docs](https://docs.rs/opentelemetry/)
- [Tracing Crate Docs](https://docs.rs/tracing/)
- [W3C Trace Context Spec](https://www.w3.org/TR/trace-context/)
- [OpenTelemetry Semantic Conventions](https://opentelemetry.io/docs/specs/semconv/)

---

_Last Updated: 2025-12-25_

