# Mizuchi Uploadr Examples

This folder contains everything you need to run a full-featured Mizuchi Uploadr environment locally.

## Quick Start

```bash
# 1. Start MinIO (S3-compatible storage)
docker-compose up -d

# 2. Generate a JWT token for authenticated uploads
./generate-jwt.sh

# 3. Upload a file (directly to MinIO)
pip install requests  # if not installed
./uploader.py upload myfile.txt /uploads/myfile.txt --endpoint http://localhost:9000
```

## Components

### Docker Compose Environment

The `docker-compose.yml` provides:

| Service | Port | Description |
|---------|------|-------------|
| MinIO | 9000 (S3), 9001 (Console) | S3-compatible object storage |
| Mizuchi | 8080 (API), 9090 (Metrics) | Upload proxy (optional) |
| Prometheus | 9091 (optional) | Metrics collection |
| Grafana | 3000 (optional) | Metrics visualization |

**Start MinIO only (for testing uploader):**
```bash
docker-compose up -d
```

**Start full environment with Mizuchi proxy:**
```bash
docker-compose --profile full up -d
```

**Start with monitoring:**
```bash
docker-compose --profile full --profile monitoring up -d
```

**View MinIO Console:**
Open http://localhost:9001 (login: `minioadmin` / `minioadmin123`)

### Configuration (`config.yaml`)

Two bucket configurations are provided:

| Bucket | Path Prefix | Auth Required |
|--------|-------------|---------------|
| Public | `/uploads` | No |
| Private | `/private` | JWT required |

### JWT Token Generator (`generate-jwt.sh`)

Generate JWT tokens for authenticated uploads:

```bash
# Default token (valid for 24 hours)
./generate-jwt.sh

# Custom user and expiry
./generate-jwt.sh --subject admin@example.com --expiry 48

# Using environment variable
JWT_SECRET=mysecret ./generate-jwt.sh -u testuser
```

**Options:**
- `-s, --secret SECRET` - JWT secret (default: from `JWT_SECRET` env)
- `-u, --subject SUBJECT` - Subject claim (default: user@example.com)
- `-e, --expiry HOURS` - Expiry in hours (default: 24)
- `-i, --issuer ISSUER` - Issuer claim (optional)
- `-a, --audience AUD` - Audience claim (optional)

### CLI Uploader (`uploader.py`)

Python CLI for uploading files with parallel chunk support.

**Requirements:**
```bash
pip install requests
```

**Simple upload (small files):**
```bash
./uploader.py upload photo.jpg /uploads/photo.jpg
```

**Authenticated upload:**
```bash
./uploader.py upload document.pdf /private/document.pdf --token <jwt>
```

**Large file with parallel chunks:**
```bash
./uploader.py upload video.mp4 /private/video.mp4 \
    --token <jwt> \
    --chunk-size 20M \
    --parallel 8
```

**Options:**
- `-e, --endpoint URL` - Server endpoint (default: http://localhost:8080)
- `-t, --token TOKEN` - JWT token for authentication
- `-c, --chunk-size SIZE` - Chunk size for multipart (default: 10M)
- `-T, --threshold SIZE` - Multipart threshold (default: 50M)
- `-p, --parallel N` - Parallel uploads (default: 4)
- `-v, --verbose` - Enable debug output

**Environment variables:**
- `MIZUCHI_ENDPOINT` - Default endpoint URL
- `MIZUCHI_TOKEN` - Default JWT token

## Example Workflows

### 1. Upload to MinIO (No Mizuchi Proxy)

```bash
# Start MinIO
docker-compose up -d

# Upload to public bucket (MinIO on port 9000)
./uploader.py upload test.txt /uploads/test.txt --endpoint http://localhost:9000
```

### 2. Upload via Mizuchi Proxy

```bash
# Start full environment
docker-compose --profile full up -d

# Upload to public bucket (Mizuchi on port 8080)
./uploader.py upload test.txt /uploads/test.txt --endpoint http://localhost:8080

# Authenticated upload to private bucket
TOKEN=$(./generate-jwt.sh 2>/dev/null | tail -1)
./uploader.py upload secret.pdf /private/secret.pdf --token "$TOKEN"
```

### 3. Large File Upload with Parallel Chunks

```bash
# Create a test file (100MB)
dd if=/dev/zero of=largefile.bin bs=1M count=100

# Upload with parallel chunks
./uploader.py upload largefile.bin /uploads/largefile.bin \
    --endpoint http://localhost:9000 \
    --chunk-size 10M \
    --parallel 4 \
    --verbose
```

### 4. Using curl

```bash
# Simple upload to MinIO
curl -X PUT http://localhost:9000/uploads/test.txt \
    -d "Hello, World!"

# Via Mizuchi proxy (if running with --profile full)
TOKEN=$(./generate-jwt.sh 2>/dev/null | tail -1)
curl -X PUT http://localhost:8080/private/test.txt \
    -H "Authorization: Bearer $TOKEN" \
    -d "Secret content"
```

## Troubleshooting

### "Connection refused" error
```bash
# Check if services are running
docker-compose ps

# View logs
docker-compose logs mizuchi
```

### "Unauthorized" error
```bash
# Ensure token is not expired
./generate-jwt.sh  # Generate a fresh token

# Check if using correct bucket
# /uploads = no auth, /private = requires auth
```

### Upload fails for large file
```bash
# Ensure chunk size is at least 5MB (S3 requirement)
./uploader.py upload file.bin /uploads/file.bin --chunk-size 10M

# Check verbose output for details
./uploader.py upload file.bin /uploads/file.bin --verbose
```

## Cleanup

```bash
# Stop all services
docker-compose down

# Remove volumes (delete data)
docker-compose down -v
```
