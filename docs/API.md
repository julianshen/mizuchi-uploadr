# Mizuchi Uploadr API Reference

Complete API reference for Mizuchi Uploadr's S3-compatible upload proxy.

## Table of Contents

- [Overview](#overview)
- [Authentication](#authentication)
- [Authorization](#authorization)
- [S3 Operations](#s3-operations)
- [Health & Metrics](#health--metrics)
- [Error Responses](#error-responses)

---

## Overview

Mizuchi Uploadr exposes an S3-compatible REST API for upload operations only. It acts as a secure proxy between clients and your S3 backend (AWS S3, MinIO, etc.).

### Base URL

```
http://localhost:8080
```

### Request Flow

```
Client → Mizuchi Uploadr → S3 Backend
         (auth/authz)
```

### Content Types

- Request bodies: `application/octet-stream` or any valid content type
- Response bodies: `text/plain` or `application/xml` (S3 responses)

---

## Authentication

Mizuchi Uploadr supports multiple authentication methods, configurable per bucket.

### JWT Authentication (Bearer Token)

**Header Format:**
```
Authorization: Bearer <token>
```

**Supported Algorithms:**
- `HS256` - HMAC-SHA256 (symmetric)
- `RS256` - RSA-SHA256 (asymmetric)
- `ES256` - ECDSA P-256 (asymmetric)

**Required Claims:**
| Claim | Type | Description |
|-------|------|-------------|
| `sub` | string | Subject (user identifier) |
| `exp` | number | Expiration timestamp (Unix epoch) |

**Optional Claims:**
| Claim | Type | Description |
|-------|------|-------------|
| `iat` | number | Issued-at timestamp |
| `iss` | string | Issuer (validated if configured) |
| `aud` | string | Audience (validated if configured) |

**Example Request:**
```bash
curl -X PUT http://localhost:8080/uploads/file.txt \
  -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9..." \
  -H "Content-Type: text/plain" \
  -d "Hello, World!"
```

**Configuration:**
```yaml
buckets:
  - name: "uploads"
    path_prefix: "/uploads"
    auth:
      enabled: true
      jwt:
        algorithm: "HS256"
        secret: "${JWT_SECRET}"
        issuer: "https://auth.example.com"  # Optional
        audience: "mizuchi-uploadr"          # Optional
```

**JWKS Support:**

For RS256/ES256, you can use a JWKS endpoint instead of a static key:

```yaml
auth:
  jwt:
    algorithm: "RS256"
    jwks_url: "https://auth.example.com/.well-known/jwks.json"
    cache_ttl_seconds: 3600  # Key cache duration
```

### AWS SigV4 Authentication

**Header Format:**
```
Authorization: AWS4-HMAC-SHA256 Credential=<access-key>/<date>/<region>/s3/aws4_request, SignedHeaders=<headers>, Signature=<signature>
X-Amz-Date: <ISO8601 timestamp>
X-Amz-Content-SHA256: <content hash>
```

**Example Request:**
```bash
# Using AWS CLI (automatically signs requests)
aws s3 cp file.txt s3://my-bucket/file.txt \
  --endpoint-url http://localhost:8080
```

**Configuration:**
```yaml
buckets:
  - name: "uploads"
    path_prefix: "/uploads"
    auth:
      enabled: true
      sigv4:
        access_key: "${AWS_ACCESS_KEY_ID}"
        secret_key: "${AWS_SECRET_ACCESS_KEY}"
        region: "us-east-1"
        max_clock_skew_seconds: 300  # Optional, default 5 minutes
```

### Query String Token

JWT tokens can also be passed via query string (useful for presigned URLs):

```
PUT /uploads/file.txt?token=eyJhbGciOiJIUzI1NiIs...
```

### Authentication Errors

| Status | Error | Description |
|--------|-------|-------------|
| `401 Unauthorized` | Missing authentication | No auth header provided |
| `401 Unauthorized` | Token expired | JWT `exp` claim is past |
| `401 Unauthorized` | Invalid token | JWT signature verification failed |
| `401 Unauthorized` | Invalid signature | SigV4 signature mismatch |

---

## Authorization

After authentication, Mizuchi Uploadr can enforce fine-grained authorization using OPA or OpenFGA.

### OPA (Open Policy Agent)

OPA evaluates Rego policies to authorize requests.

**Policy Input:**
```json
{
  "input": {
    "subject": "user123",
    "action": "upload",
    "resource": {
      "bucket": "uploads",
      "key": "path/to/file.txt"
    },
    "context": {
      "method": "PUT",
      "path": "/uploads/path/to/file.txt",
      "ip": "192.168.1.100"
    }
  }
}
```

**Example Policy (Rego):**
```rego
package mizuchi

default allow = false

# Allow uploads to /public/* for any authenticated user
allow {
  input.action == "upload"
  startswith(input.resource.key, "public/")
}

# Allow admins to upload anywhere
allow {
  input.subject == "admin"
}
```

**Configuration:**
```yaml
buckets:
  - name: "uploads"
    authz:
      enabled: true
      opa:
        url: "http://localhost:8181"
        policy_path: "/v1/data/mizuchi/allow"
        timeout_seconds: 5
        cache_ttl_seconds: 60
```

### OpenFGA

OpenFGA provides relationship-based access control.

**Relationship Model:**
```
user:alice -> writer -> bucket:uploads
user:bob -> reader -> bucket:uploads
```

**Check Request:**
```json
{
  "user": "user:alice",
  "relation": "writer",
  "object": "bucket:uploads"
}
```

**Configuration:**
```yaml
buckets:
  - name: "uploads"
    authz:
      enabled: true
      openfga:
        url: "http://localhost:8080"
        store_id: "${OPENFGA_STORE_ID}"
        model_id: "${OPENFGA_MODEL_ID}"  # Optional
        timeout_seconds: 5
        cache_ttl_seconds: 60
```

### Authorization Errors

| Status | Error | Description |
|--------|-------|-------------|
| `403 Forbidden` | Access denied | Authorization policy rejected request |
| `500 Internal Server Error` | Authorization error | OPA/OpenFGA service unavailable |

---

## S3 Operations

### PutObject

Upload a single object (files up to 5GB).

**Request:**
```
PUT /{path_prefix}/{key}
Authorization: Bearer <token>
Content-Type: <content-type>
Content-Length: <size>

<body>
```

**Example:**
```bash
curl -X PUT http://localhost:8080/uploads/documents/report.pdf \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/pdf" \
  --data-binary @report.pdf
```

**Response (Success):**
```
HTTP/1.1 200 OK
ETag: "d41d8cd98f00b204e9800998ecf8427e"
Content-Type: text/plain

Upload successful
```

**Response Headers:**
| Header | Description |
|--------|-------------|
| `ETag` | MD5 hash of uploaded content |

### CreateMultipartUpload

Initiate a multipart upload for large files (>50MB recommended).

**Request:**
```
POST /{path_prefix}/{key}?uploads
Authorization: Bearer <token>
Content-Type: <content-type>
```

**Example:**
```bash
curl -X POST "http://localhost:8080/uploads/large-file.bin?uploads" \
  -H "Authorization: Bearer $TOKEN" \
  -H "Content-Type: application/octet-stream"
```

**Response:**
```xml
<?xml version="1.0" encoding="UTF-8"?>
<InitiateMultipartUploadResult>
  <Bucket>my-bucket</Bucket>
  <Key>large-file.bin</Key>
  <UploadId>abc123...</UploadId>
</InitiateMultipartUploadResult>
```

### UploadPart

Upload a part of a multipart upload (minimum 5MB except last part).

**Request:**
```
PUT /{path_prefix}/{key}?partNumber={n}&uploadId={id}
Authorization: Bearer <token>
Content-Length: <size>

<part-body>
```

**Example:**
```bash
curl -X PUT "http://localhost:8080/uploads/large-file.bin?partNumber=1&uploadId=abc123" \
  -H "Authorization: Bearer $TOKEN" \
  --data-binary @part1.bin
```

**Response:**
```
HTTP/1.1 200 OK
ETag: "part-etag-here"
```

### CompleteMultipartUpload

Complete a multipart upload by combining all parts.

**Request:**
```
POST /{path_prefix}/{key}?uploadId={id}
Authorization: Bearer <token>
Content-Type: application/xml

<CompleteMultipartUpload>
  <Part>
    <PartNumber>1</PartNumber>
    <ETag>"part1-etag"</ETag>
  </Part>
  <Part>
    <PartNumber>2</PartNumber>
    <ETag>"part2-etag"</ETag>
  </Part>
</CompleteMultipartUpload>
```

**Response:**
```xml
<?xml version="1.0" encoding="UTF-8"?>
<CompleteMultipartUploadResult>
  <Location>http://bucket.s3.amazonaws.com/key</Location>
  <Bucket>my-bucket</Bucket>
  <Key>large-file.bin</Key>
  <ETag>"composite-etag"</ETag>
</CompleteMultipartUploadResult>
```

### AbortMultipartUpload

Cancel a multipart upload and delete uploaded parts.

**Request:**
```
DELETE /{path_prefix}/{key}?uploadId={id}
Authorization: Bearer <token>
```

**Example:**
```bash
curl -X DELETE "http://localhost:8080/uploads/large-file.bin?uploadId=abc123" \
  -H "Authorization: Bearer $TOKEN"
```

**Response:**
```
HTTP/1.1 204 No Content
```

### ListParts

List uploaded parts for a multipart upload.

**Request:**
```
GET /{path_prefix}/{key}?uploadId={id}
Authorization: Bearer <token>
```

**Response:**
```xml
<?xml version="1.0" encoding="UTF-8"?>
<ListPartsResult>
  <Bucket>my-bucket</Bucket>
  <Key>large-file.bin</Key>
  <UploadId>abc123</UploadId>
  <Part>
    <PartNumber>1</PartNumber>
    <ETag>"etag1"</ETag>
    <Size>5242880</Size>
  </Part>
</ListPartsResult>
```

---

## Health & Metrics

### Health Check

**Request:**
```
GET /health
```

**Response:**
```
HTTP/1.1 200 OK
Content-Type: text/plain

ok
```

### Prometheus Metrics

**Request:**
```
GET /metrics
```

(Available on metrics port, default 9090)

**Response:**
```
# HELP mizuchi_uploads_total Total number of uploads
# TYPE mizuchi_uploads_total counter
mizuchi_uploads_total{bucket="uploads",status="success"} 1234

# HELP mizuchi_upload_bytes_total Total bytes uploaded
# TYPE mizuchi_upload_bytes_total counter
mizuchi_upload_bytes_total{bucket="uploads"} 12345678

# HELP mizuchi_upload_duration_seconds Upload duration in seconds
# TYPE mizuchi_upload_duration_seconds histogram
mizuchi_upload_duration_seconds_bucket{bucket="uploads",le="0.1"} 100
...
```

**Key Metrics:**
| Metric | Type | Description |
|--------|------|-------------|
| `mizuchi_uploads_total` | counter | Total uploads (by bucket, status) |
| `mizuchi_upload_bytes_total` | counter | Total bytes uploaded |
| `mizuchi_upload_duration_seconds` | histogram | Upload latency |
| `mizuchi_multipart_uploads_total` | counter | Multipart uploads |
| `mizuchi_auth_requests_total` | counter | Auth requests (by method, result) |
| `mizuchi_zero_copy_bytes_total` | counter | Bytes transferred via zero-copy |

---

## Error Responses

### Standard Error Format

```
HTTP/1.1 <status>
Content-Type: text/plain

<error message>
```

### Error Codes

| Status | Error | Description |
|--------|-------|-------------|
| `400 Bad Request` | Invalid key | Object key is empty or invalid |
| `400 Bad Request` | Failed to read body | Request body unreadable |
| `401 Unauthorized` | Missing authentication | No auth header |
| `401 Unauthorized` | Token expired | JWT expired |
| `401 Unauthorized` | Invalid token | JWT verification failed |
| `403 Forbidden` | Access denied | Authorization rejected |
| `404 Not Found` | Not Found | Path doesn't match any bucket |
| `500 Internal Server Error` | Server configuration error | Misconfigured auth |
| `500 Internal Server Error` | Upload failed | S3 backend error |
| `500 Internal Server Error` | Failed to create S3 client | S3 connection issue |

### S3 Error Responses

When the S3 backend returns an error, it's forwarded as:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<Error>
  <Code>NoSuchBucket</Code>
  <Message>The specified bucket does not exist</Message>
  <BucketName>nonexistent</BucketName>
  <RequestId>request-id</RequestId>
</Error>
```

---

## Rate Limiting

Mizuchi Uploadr does not currently implement rate limiting. For production deployments, consider:

1. **External rate limiting** - Use a load balancer (nginx, HAProxy, AWS ALB)
2. **S3 throttling** - S3 itself enforces request limits per prefix

---

## CORS

CORS is not currently implemented. For browser uploads, consider:

1. **Presigned URLs** - Generate presigned S3 URLs on your backend
2. **Proxy configuration** - Add CORS headers via reverse proxy

---

## SDK Compatibility

Mizuchi Uploadr is compatible with:

- **AWS SDK** (all languages)
- **MinIO Client** (mc)
- **s3cmd**
- **rclone**
- **Any S3-compatible client**

**Example with AWS CLI:**
```bash
aws s3 cp file.txt s3://uploads/file.txt \
  --endpoint-url http://localhost:8080
```

**Example with boto3 (Python):**
```python
import boto3

s3 = boto3.client('s3',
    endpoint_url='http://localhost:8080',
    aws_access_key_id='your-key',
    aws_secret_access_key='your-secret'
)

s3.upload_file('local-file.txt', 'uploads', 'remote-file.txt')
```

---

## Further Reading

- [Configuration Reference](CONFIG.md)
- [Tracing Guide](TRACING.md)
- [Deployment Guide](DEPLOYMENT.md)
- [AWS S3 REST API Reference](https://docs.aws.amazon.com/AmazonS3/latest/API/)
