# Mizuchi Uploadr Examples

This folder contains everything you need to run a full-featured Mizuchi Uploadr environment locally.

## Quick Start

```bash
# 1. Start the environment
docker-compose up -d

# 2. Generate a JWT token for authenticated uploads
./generate-jwt.sh

# 3. Upload a file
./uploader.py upload myfile.txt /uploads/myfile.txt
```

## Components

### Docker Compose Environment

The `docker-compose.yml` provides:

| Service | Port | Description |
|---------|------|-------------|
| MinIO | 9000 (S3), 9001 (Console) | S3-compatible object storage |
| Mizuchi | 8080 (API), 9090 (Metrics) | Upload proxy |
| Prometheus | 9091 (optional) | Metrics collection |
| Grafana | 3000 (optional) | Metrics visualization |

**Start core services:**
```bash
docker-compose up -d
```

**Start with monitoring:**
```bash
docker-compose --profile monitoring up -d
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

### 1. Public Upload (No Auth)

```bash
# Start services
docker-compose up -d

# Upload to public bucket
./uploader.py upload test.txt /uploads/test.txt
```

### 2. Authenticated Upload

```bash
# Generate token
TOKEN=$(./generate-jwt.sh 2>/dev/null | tail -1)

# Upload to private bucket
./uploader.py upload secret.pdf /private/secret.pdf --token "$TOKEN"
```

### 3. Large File Upload

```bash
# Create a test file (100MB)
dd if=/dev/zero of=largefile.bin bs=1M count=100

# Upload with parallel chunks
./uploader.py upload largefile.bin /uploads/largefile.bin \
    --chunk-size 10M \
    --parallel 4 \
    --verbose
```

### 4. Using curl

```bash
# Simple upload
curl -X PUT http://localhost:8080/uploads/test.txt \
    -d "Hello, World!"

# Authenticated upload
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
