# Deployment Guide

This guide covers deploying Mizuchi Uploadr in various environments.

## Table of Contents

- [Prerequisites](#prerequisites)
- [Docker Deployment](#docker-deployment)
- [Kubernetes Deployment](#kubernetes-deployment)
- [Bare Metal Deployment](#bare-metal-deployment)
- [Configuration Best Practices](#configuration-best-practices)
- [Security Hardening](#security-hardening)
- [Monitoring & Observability](#monitoring--observability)
- [Troubleshooting](#troubleshooting)

---

## Prerequisites

### System Requirements

| Component | Minimum | Recommended |
|-----------|---------|-------------|
| CPU | 1 core | 2+ cores |
| Memory | 256 MB | 512 MB+ |
| Disk | 100 MB | 1 GB+ (for temp files) |
| Network | 100 Mbps | 1 Gbps+ |

### Software Requirements

- **Linux**: Kernel 2.6.17+ (for zero-copy splice)
- **Docker**: 20.10+ (for container deployment)
- **Kubernetes**: 1.21+ (for K8s deployment)

### S3 Backend

Ensure your S3 backend is accessible:
- AWS S3
- MinIO
- Cloudflare R2
- DigitalOcean Spaces
- Any S3-compatible service

---

## Docker Deployment

### Quick Start

```bash
# Pull the image
docker pull ghcr.io/julianshen/mizuchi-uploadr:latest

# Run with config file
docker run -d \
  --name mizuchi \
  -p 8080:8080 \
  -p 9090:9090 \
  -v $(pwd)/config.yaml:/etc/mizuchi/config.yaml \
  ghcr.io/julianshen/mizuchi-uploadr:latest
```

### Docker Compose

```yaml
# docker-compose.yaml
version: "3.8"

services:
  mizuchi:
    image: ghcr.io/julianshen/mizuchi-uploadr:latest
    ports:
      - "8080:8080"   # Main API
      - "9090:9090"   # Metrics
    volumes:
      - ./config.yaml:/etc/mizuchi/config.yaml:ro
    environment:
      - AWS_ACCESS_KEY_ID=${AWS_ACCESS_KEY_ID}
      - AWS_SECRET_ACCESS_KEY=${AWS_SECRET_ACCESS_KEY}
      - JWT_SECRET=${JWT_SECRET}
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8080/health"]
      interval: 30s
      timeout: 10s
      retries: 3
    restart: unless-stopped
```

### Docker Compose with MinIO (Development)

```yaml
# docker-compose.dev.yaml
version: "3.8"

services:
  mizuchi:
    image: ghcr.io/julianshen/mizuchi-uploadr:latest
    ports:
      - "8080:8080"
      - "9090:9090"
    volumes:
      - ./config.yaml:/etc/mizuchi/config.yaml:ro
    environment:
      - AWS_ACCESS_KEY_ID=minioadmin
      - AWS_SECRET_ACCESS_KEY=minioadmin
    depends_on:
      minio:
        condition: service_healthy

  minio:
    image: minio/minio:latest
    ports:
      - "9000:9000"
      - "9001:9001"
    environment:
      - MINIO_ROOT_USER=minioadmin
      - MINIO_ROOT_PASSWORD=minioadmin
    command: server /data --console-address ":9001"
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:9000/minio/health/live"]
      interval: 30s
      timeout: 20s
      retries: 3
    volumes:
      - minio-data:/data

  # Create bucket on startup
  minio-init:
    image: minio/mc:latest
    depends_on:
      minio:
        condition: service_healthy
    entrypoint: >
      /bin/sh -c "
      mc alias set myminio http://minio:9000 minioadmin minioadmin;
      mc mb --ignore-existing myminio/uploads;
      exit 0;
      "

volumes:
  minio-data:
```

### Building Custom Image

```dockerfile
# Dockerfile
FROM rust:1.75 as builder

WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/mizuchi-uploadr /usr/local/bin/

EXPOSE 8080 9090

ENTRYPOINT ["mizuchi-uploadr"]
CMD ["--config", "/etc/mizuchi/config.yaml"]
```

---

## Kubernetes Deployment

### ConfigMap

```yaml
# configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: mizuchi-config
data:
  config.yaml: |
    server:
      address: "0.0.0.0:8080"
      zero_copy:
        enabled: true

    buckets:
      - name: "uploads"
        path_prefix: "/uploads"
        s3:
          bucket: "${S3_BUCKET}"
          region: "${AWS_REGION}"
          access_key: "${AWS_ACCESS_KEY_ID}"
          secret_key: "${AWS_SECRET_ACCESS_KEY}"
        auth:
          enabled: true
          jwt:
            secret: "${JWT_SECRET}"

    metrics:
      enabled: true
      port: 9090
```

### Secret

```yaml
# secret.yaml
apiVersion: v1
kind: Secret
metadata:
  name: mizuchi-secrets
type: Opaque
stringData:
  AWS_ACCESS_KEY_ID: "your-access-key"
  AWS_SECRET_ACCESS_KEY: "your-secret-key"
  JWT_SECRET: "your-jwt-secret"
  S3_BUCKET: "your-bucket"
  AWS_REGION: "us-east-1"
```

### Deployment

```yaml
# deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: mizuchi-uploadr
  labels:
    app: mizuchi-uploadr
spec:
  replicas: 3
  selector:
    matchLabels:
      app: mizuchi-uploadr
  template:
    metadata:
      labels:
        app: mizuchi-uploadr
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "9090"
    spec:
      containers:
        - name: mizuchi
          image: ghcr.io/julianshen/mizuchi-uploadr:latest
          ports:
            - name: http
              containerPort: 8080
            - name: metrics
              containerPort: 9090
          envFrom:
            - secretRef:
                name: mizuchi-secrets
          volumeMounts:
            - name: config
              mountPath: /etc/mizuchi
              readOnly: true
          resources:
            requests:
              cpu: "100m"
              memory: "256Mi"
            limits:
              cpu: "1000m"
              memory: "512Mi"
          livenessProbe:
            httpGet:
              path: /health
              port: http
            initialDelaySeconds: 5
            periodSeconds: 10
          readinessProbe:
            httpGet:
              path: /health
              port: http
            initialDelaySeconds: 5
            periodSeconds: 5
      volumes:
        - name: config
          configMap:
            name: mizuchi-config
```

### Service

```yaml
# service.yaml
apiVersion: v1
kind: Service
metadata:
  name: mizuchi-uploadr
spec:
  selector:
    app: mizuchi-uploadr
  ports:
    - name: http
      port: 80
      targetPort: 8080
    - name: metrics
      port: 9090
      targetPort: 9090
  type: ClusterIP
```

### Ingress

```yaml
# ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: mizuchi-uploadr
  annotations:
    nginx.ingress.kubernetes.io/proxy-body-size: "0"  # Unlimited
    nginx.ingress.kubernetes.io/proxy-read-timeout: "600"
spec:
  ingressClassName: nginx
  rules:
    - host: uploads.example.com
      http:
        paths:
          - path: /
            pathType: Prefix
            backend:
              service:
                name: mizuchi-uploadr
                port:
                  number: 80
  tls:
    - hosts:
        - uploads.example.com
      secretName: uploads-tls
```

### HorizontalPodAutoscaler

```yaml
# hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: mizuchi-uploadr
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: Deployment
    name: mizuchi-uploadr
  minReplicas: 2
  maxReplicas: 10
  metrics:
    - type: Resource
      resource:
        name: cpu
        target:
          type: Utilization
          averageUtilization: 70
```

### Apply All Resources

```bash
kubectl apply -f configmap.yaml
kubectl apply -f secret.yaml
kubectl apply -f deployment.yaml
kubectl apply -f service.yaml
kubectl apply -f ingress.yaml
kubectl apply -f hpa.yaml
```

---

## Bare Metal Deployment

### Systemd Service

```ini
# /etc/systemd/system/mizuchi-uploadr.service
[Unit]
Description=Mizuchi Uploadr S3 Proxy
After=network.target

[Service]
Type=simple
User=mizuchi
Group=mizuchi
ExecStart=/usr/local/bin/mizuchi-uploadr --config /etc/mizuchi/config.yaml
Restart=always
RestartSec=5
Environment=RUST_LOG=info

# Security hardening
NoNewPrivileges=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/var/lib/mizuchi
PrivateTmp=true

[Install]
WantedBy=multi-user.target
```

### Installation Steps

```bash
# 1. Create user
sudo useradd -r -s /bin/false mizuchi

# 2. Install binary
sudo cp target/release/mizuchi-uploadr /usr/local/bin/
sudo chmod +x /usr/local/bin/mizuchi-uploadr

# 3. Create directories
sudo mkdir -p /etc/mizuchi /var/lib/mizuchi
sudo chown mizuchi:mizuchi /var/lib/mizuchi

# 4. Copy configuration
sudo cp config.yaml /etc/mizuchi/
sudo chmod 600 /etc/mizuchi/config.yaml
sudo chown mizuchi:mizuchi /etc/mizuchi/config.yaml

# 5. Enable and start service
sudo systemctl daemon-reload
sudo systemctl enable mizuchi-uploadr
sudo systemctl start mizuchi-uploadr

# 6. Check status
sudo systemctl status mizuchi-uploadr
```

### Nginx Reverse Proxy

```nginx
# /etc/nginx/sites-available/mizuchi
upstream mizuchi {
    server 127.0.0.1:8080;
    keepalive 32;
}

server {
    listen 443 ssl http2;
    server_name uploads.example.com;

    ssl_certificate /etc/letsencrypt/live/uploads.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/uploads.example.com/privkey.pem;

    # Allow unlimited body size for uploads
    client_max_body_size 0;

    # Increase timeouts for large uploads
    proxy_read_timeout 600s;
    proxy_send_timeout 600s;

    location / {
        proxy_pass http://mizuchi;
        proxy_http_version 1.1;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header Connection "";

        # Don't buffer uploads
        proxy_request_buffering off;
    }
}
```

---

## Configuration Best Practices

### Environment-Specific Configs

```
config/
├── base.yaml           # Common settings
├── development.yaml    # Dev overrides
├── staging.yaml        # Staging overrides
└── production.yaml     # Production overrides
```

### Secrets Management

**Never commit secrets!** Use:

1. **Environment variables**
2. **Kubernetes Secrets**
3. **HashiCorp Vault**
4. **AWS Secrets Manager**
5. **Docker Secrets**

```yaml
# Good: Reference environment variable
jwt:
  secret: "${JWT_SECRET}"

# Bad: Hardcoded secret
jwt:
  secret: "my-super-secret-key"
```

### Resource Limits

```yaml
# Kubernetes resource limits
resources:
  requests:
    cpu: "100m"      # 0.1 CPU
    memory: "256Mi"
  limits:
    cpu: "2000m"     # 2 CPUs (for zero-copy performance)
    memory: "512Mi"
```

---

## Security Hardening

### Network Security

1. **Use TLS** - Always terminate TLS at load balancer or reverse proxy
2. **Firewall** - Only expose ports 8080 (API) and 9090 (metrics)
3. **Private networks** - Keep S3 backend traffic internal

### Authentication

1. **Use asymmetric JWT** - RS256/ES256 over HS256
2. **Short token expiry** - 15-60 minutes
3. **Rotate secrets** - Regular key rotation

### Authorization

1. **Least privilege** - Only grant necessary permissions
2. **Audit logging** - Log all authorization decisions
3. **Separate buckets** - Different buckets for different access levels

### Container Security

```yaml
# Kubernetes SecurityContext
securityContext:
  runAsNonRoot: true
  runAsUser: 65534
  readOnlyRootFilesystem: true
  allowPrivilegeEscalation: false
  capabilities:
    drop:
      - ALL
```

---

## Monitoring & Observability

### Prometheus Metrics

```yaml
# prometheus.yaml scrape config
scrape_configs:
  - job_name: 'mizuchi-uploadr'
    static_configs:
      - targets: ['mizuchi:9090']
```

### Key Metrics to Monitor

| Metric | Description | Alert Threshold |
|--------|-------------|-----------------|
| `mizuchi_uploads_total{status="error"}` | Failed uploads | > 1% of total |
| `mizuchi_upload_duration_seconds` | Upload latency | p99 > 30s |
| `mizuchi_auth_requests_total{result="denied"}` | Auth failures | Spike detection |

### Grafana Dashboard

Import the provided dashboard from `grafana/mizuchi-dashboard.json`:

```bash
curl -X POST \
  -H "Content-Type: application/json" \
  -d @grafana/mizuchi-dashboard.json \
  http://localhost:3000/api/dashboards/db
```

### Alerting Rules

```yaml
# prometheus-rules.yaml
groups:
  - name: mizuchi
    rules:
      - alert: MizuchiHighErrorRate
        expr: rate(mizuchi_uploads_total{status="error"}[5m]) > 0.01
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High upload error rate"

      - alert: MizuchiHighLatency
        expr: histogram_quantile(0.99, mizuchi_upload_duration_seconds_bucket) > 30
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High upload latency"
```

### Distributed Tracing

See [TRACING.md](TRACING.md) for complete tracing setup.

```yaml
tracing:
  enabled: true
  otlp:
    endpoint: "http://tempo:4317"
  sampling:
    strategy: "ratio"
    ratio: 0.1  # 10% in production
```

---

## Troubleshooting

### Common Issues

#### Upload Timeouts

**Symptom**: Large uploads timeout

**Solution**:
```yaml
# Increase proxy timeouts
# nginx.conf
proxy_read_timeout 600s;

# Or Kubernetes ingress
nginx.ingress.kubernetes.io/proxy-read-timeout: "600"
```

#### Memory Issues

**Symptom**: OOM kills during large uploads

**Solution**:
```yaml
# Reduce concurrent uploads
upload:
  concurrent_parts: 2  # Reduce from 4
```

#### Zero-Copy Not Working

**Symptom**: Slow uploads on Linux

**Check**:
```bash
# Verify kernel version
uname -r  # Must be >= 2.6.17

# Check splice support
cat /proc/sys/fs/pipe-max-size
```

#### S3 Connection Errors

**Symptom**: "Failed to create S3 client"

**Check**:
```bash
# Test S3 connectivity
aws s3 ls --endpoint-url http://your-endpoint

# Check credentials
aws sts get-caller-identity
```

### Debug Logging

```bash
# Enable debug logging
RUST_LOG=debug mizuchi-uploadr --config config.yaml

# Or specific module
RUST_LOG=mizuchi_uploadr::s3=debug mizuchi-uploadr --config config.yaml
```

### Health Checks

```bash
# Check health endpoint
curl http://localhost:8080/health

# Check metrics
curl http://localhost:9090/metrics | grep mizuchi_
```

---

## Further Reading

- [API Reference](API.md)
- [Configuration Reference](CONFIG.md)
- [Tracing Guide](TRACING.md)
