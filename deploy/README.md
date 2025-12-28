# Mizuchi Uploadr Deployment

This directory contains deployment configurations for Mizuchi Uploadr.

## Directory Structure

```
deploy/
├── helm/                    # Helm chart
│   └── mizuchi-uploadr/
│       ├── Chart.yaml
│       ├── values.yaml
│       └── templates/
└── kustomize/               # Kustomize configurations
    ├── base/                # Base resources
    └── overlays/
        ├── dev/             # Development environment
        ├── staging/         # Staging environment
        └── production/      # Production environment
```

## Helm Chart

### Quick Start

```bash
# Add the chart repository (if published)
helm repo add mizuchi https://julianshen.github.io/mizuchi-uploadr

# Install with default values
helm install mizuchi-uploadr ./deploy/helm/mizuchi-uploadr

# Install with custom values
helm install mizuchi-uploadr ./deploy/helm/mizuchi-uploadr \
  --set secrets.awsAccessKeyId=YOUR_KEY \
  --set secrets.awsSecretAccessKey=YOUR_SECRET \
  --set config.buckets[0].s3.bucket=my-bucket

# Install with values file
helm install mizuchi-uploadr ./deploy/helm/mizuchi-uploadr \
  -f my-values.yaml
```

### Configuration

See `helm/mizuchi-uploadr/values.yaml` for all available options.

Key configurations:

| Parameter | Description | Default |
|-----------|-------------|---------|
| `replicaCount` | Number of replicas | `1` |
| `image.repository` | Image repository | `ghcr.io/julianshen/mizuchi-uploadr` |
| `image.tag` | Image tag | Chart appVersion |
| `config.buckets` | Bucket configurations | See values.yaml |
| `secrets.awsAccessKeyId` | AWS access key | `""` |
| `secrets.awsSecretAccessKey` | AWS secret key | `""` |
| `ingress.enabled` | Enable ingress | `false` |
| `autoscaling.enabled` | Enable HPA | `false` |

### Upgrading

```bash
helm upgrade mizuchi-uploadr ./deploy/helm/mizuchi-uploadr -f my-values.yaml
```

### Uninstalling

```bash
helm uninstall mizuchi-uploadr
```

## Kustomize

### Quick Start

```bash
# Deploy to dev namespace
kubectl apply -k deploy/kustomize/overlays/dev

# Deploy to production namespace
kubectl apply -k deploy/kustomize/overlays/production
```

### Environments

#### Development

- Single replica
- Lower resource limits
- Zero-copy disabled (macOS compatibility)
- All traces sampled
- MinIO endpoint configured

```bash
kubectl apply -k deploy/kustomize/overlays/dev
```

#### Production

- 3 replicas minimum
- Higher resource limits
- Zero-copy enabled
- 10% trace sampling
- JWT authentication enabled
- HPA and PDB configured

```bash
# First, create the secrets
kubectl create secret generic mizuchi-uploadr-secrets \
  --namespace mizuchi-prod \
  --from-literal=AWS_ACCESS_KEY_ID=xxx \
  --from-literal=AWS_SECRET_ACCESS_KEY=xxx \
  --from-literal=JWT_SECRET=xxx \
  --from-literal=S3_BUCKET=my-bucket \
  --from-literal=AWS_REGION=us-east-1

# Then apply the manifests
kubectl apply -k deploy/kustomize/overlays/production
```

### Customization

To customize for your environment:

1. Copy an existing overlay:
   ```bash
   cp -r deploy/kustomize/overlays/dev deploy/kustomize/overlays/my-env
   ```

2. Edit the configuration files:
   - `kustomization.yaml` - Namespace, labels, image tags
   - `config.yaml` - Application configuration
   - `deployment-patch.yaml` - Resource limits, replicas
   - `secret.yaml` - Credentials (don't commit real secrets!)

3. Apply:
   ```bash
   kubectl apply -k deploy/kustomize/overlays/my-env
   ```

## Docker Image

The Docker image is published to GitHub Container Registry (GHCR).

```bash
# Latest from main branch
docker pull ghcr.io/julianshen/mizuchi-uploadr:latest

# Specific version
docker pull ghcr.io/julianshen/mizuchi-uploadr:v0.1.0

# Git SHA
docker pull ghcr.io/julianshen/mizuchi-uploadr:abc1234
```

### Building Locally

```bash
docker build -t mizuchi-uploadr .
```

## Security Considerations

1. **Secrets**: Never commit real secrets. Use:
   - Kubernetes Secrets (created out-of-band)
   - External secrets managers (Vault, AWS Secrets Manager)
   - Sealed Secrets

2. **Network Policies**: Consider adding network policies to restrict traffic.

3. **Pod Security**: The default configurations use:
   - Non-root user (65534)
   - Read-only root filesystem
   - Dropped capabilities

4. **TLS**: Always use TLS in production via Ingress or service mesh.

## Monitoring

### Prometheus

Metrics are exposed on port 9090 at `/metrics`.

For Prometheus Operator, enable ServiceMonitor:

```yaml
# Helm
serviceMonitor:
  enabled: true

# Or apply separately
kubectl apply -f - <<EOF
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: mizuchi-uploadr
spec:
  selector:
    matchLabels:
      app.kubernetes.io/name: mizuchi-uploadr
  endpoints:
    - port: metrics
      interval: 30s
EOF
```

### Tracing

Configure OTLP endpoint in the config:

```yaml
tracing:
  enabled: true
  otlp:
    endpoint: "http://jaeger:4317"
```
