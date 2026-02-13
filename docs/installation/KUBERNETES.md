# Installation on Kubernetes

**cronbird** can be easily deployed to Kubernetes using the provided Helm chart.

## Prerequisites

- [Kubernetes](https://kubernetes.io/) 1.20+
- [Helm](https://helm.sh/) 3.0+

## Installing with Helm

1. **Add the repository:**
   ```bash
   helm repo add tinyops https://tinyops.ru/helm-charts/
   helm repo update
   ```

2. **Install the chart:**
   ```bash
   helm upgrade --install cronbird tinyops/cronbird \
     --namespace cronbird \
     --create-namespace
   ```

## Configuration

You can customize the deployment by creating a `values.yaml` file or using `--set` flags.

### Example `values.yaml`

```yaml
config:
  identities: "daily-backup,weekly-cleanup"
  allowDynamic: false
  secrets:
    authToken: "your-super-secret-token"

persistence:
  enabled: true
  size: 1Gi
  storageClass: "standard"

resources:
  limits:
    cpu: 100m
    memory: 128Mi
  requests:
    cpu: 10m
    memory: 32Mi
```

Apply with:
```bash
helm upgrade --install cronbird tinyops/cronbird -f values.yaml
```

## Security Hardening

The Helm chart includes several security features enabled by default:
- **Non-root user**: Runs as UID `1001`.
- **Read-only root filesystem**: Prevents modifications to the container image at runtime.
- **Dropped capabilities**: Minimizes the attack surface.
- **Seccomp profile**: Uses `RuntimeDefault`.

## Monitoring with Prometheus

If you use Prometheus Operator, you can enable a `ServiceMonitor` in your `values.yaml`:

```yaml
serviceMonitor:
  enabled: true
  interval: 30s
  labels:
    release: prometheus-stack
```

## Persistence

For production environments, it is highly recommended to enable persistence to avoid losing monitoring state when the pod restarts.

```yaml
persistence:
  enabled: true
  accessMode: ReadWriteOnce
  size: 1Gi
```

For more detailed information about chart parameters, see the [Helm Chart README](../../helm-chart/README.md).
