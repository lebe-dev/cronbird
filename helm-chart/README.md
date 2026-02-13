# cronbird helm Chart

Helm chart for deploying [cronbird](https://github.com/lebe-dev/cronbird) - a lightweight HTTP service that monitors cron job execution through callback pings and exports metrics for Prometheus/VictoriaMetrics.

## Installation

```bash
helm repo add tinyops https://tinyops.ru/helm-charts/
helm repo update
helm upgrade --install --create-namespace -n cronbird cronbird tinyops/cronbird --version 0.1.0
```

## Uninstallation

```bash
helm uninstall -n cronbird cronbird
```

## Configuration

The following table lists the configurable parameters under `.config` section:

| Parameter | Description | Default | Required |
|-----------|-------------|---------|----------|
| `config.listen` | Listen address for the HTTP server (format: `IP:PORT`) | `"0.0.0.0:8080"` | No |
| `config.identities` | Comma-separated list of predefined job identities. Example: `"clickhouse-backup-prod,pg-backup-staging,redis-backup-prod"`. Required if `allowDynamic` is `false` | `""` | Conditional |
| `config.allowDynamic` | Allow dynamic identities (accept unknown identities on POST). Set to `true` to accept any valid identity without predefinition. Set to `false` to only accept identities listed in `identities` | `false` | No |
| `config.persistPath` | Path to state persistence file inside the container | `"/data/cronbird-state.json"` | No |
| `config.persistInterval` | Persistence interval in seconds (how often to write state to disk) | `60` | No |
| `config.logLevel` | Log level: `debug`, `info`, `warn`, or `error` | `"info"` | No |
| `config.secrets.authToken` | Bearer token for authentication. If set, all `POST /callback/*` requests must include `Authorization: Bearer <token>` header. Leave empty to disable authentication | `""` | No |
| `config.secrets.annotations` | Annotations to add to the Secret resource. Useful for external secret management systems like External Secrets Operator or Sealed Secrets | `{}` | No |

### Configuration Examples

#### Static identities with authentication

```yaml
config:
  identities: "backup-prod,backup-staging,monitoring-job"
  allowDynamic: false
  secrets:
    authToken: "my-secret-token-here"
```

#### Dynamic mode without authentication (dev/testing)

```yaml
config:
  identities: ""
  allowDynamic: true
  secrets:
    authToken: ""
```

#### With External Secrets Operator

```yaml
config:
  secrets:
    authToken: ""  # Will be injected by External Secrets
    annotations:
      external-secrets.io/backend: vault
      external-secrets.io/key-path: secret/data/cronbird/auth-token
```

#### With Sealed Secrets

```yaml
config:
  secrets:
    authToken: "AQB5tz..."  # Sealed value
    annotations:
      sealedsecrets.bitnami.com/managed: "true"
```

## Security

This Helm chart implements comprehensive security hardening following Kubernetes best practices:

### Pod Security

- **Non-root user**: Container runs as UID/GID `1001` (user `cronbird`)
- **Non-root enforcement**: `runAsNonRoot: true` prevents accidental root execution
- **Filesystem group**: `fsGroup: 1001` ensures proper volume ownership
- **Seccomp profile**: `RuntimeDefault` restricts available syscalls

### Container Security

- **Read-only root filesystem**: `readOnlyRootFilesystem: true` prevents runtime modifications
  - Writable `/data` directory mounted as volume for state persistence
- **Dropped capabilities**: All Linux capabilities dropped (`drop: [ALL]`)
- **No privilege escalation**: `allowPrivilegeEscalation: false` prevents privilege elevation
- **Seccomp profile**: Container-level `RuntimeDefault` seccomp filtering

### Secret Management

- **Optional authentication**: Bearer token authentication can be enabled via `config.secrets.authToken`
- **Constant-time comparison**: Token validation uses constant-time comparison to prevent timing attacks
- **Kubernetes Secret**: Sensitive data stored in native Secret resources
- **External secret systems**: Supports annotations for External Secrets Operator, Sealed Secrets, etc.

### Network Security

- **Health check endpoint**: `/health` endpoint for liveness/readiness probes
- **Minimal port exposure**: Only HTTP port `8080` exposed
- **Service isolation**: ClusterIP service by default (not exposed externally)

### Persistence Security

- **Optional persistence**: State persistence disabled by default (uses `emptyDir`)
- **PVC support**: Production deployments can enable PersistentVolumeClaim
- **Volume ownership**: `fsGroup` ensures proper volume permissions

### Recommendations

1. **Always enable authentication** in production:
   ```yaml
   config:
     secrets:
       authToken: "<strong-random-token>"
   ```

2. **Use external secret management** for production:
   - External Secrets Operator
   - Sealed Secrets
   - HashiCorp Vault integration

3. **Enable persistence** for production deployments:
   ```yaml
   persistence:
     enabled: true
     size: 5Gi
   ```

4. **Restrict identities** in production (disable dynamic mode):
   ```yaml
   config:
     identities: "known-job1,known-job2"
     allowDynamic: false
   ```

5. **Use NetworkPolicy** to restrict traffic (not included in chart):
   ```yaml
   apiVersion: networking.k8s.io/v1
   kind: NetworkPolicy
   metadata:
     name: cronbird-netpol
   spec:
     podSelector:
       matchLabels:
         app.kubernetes.io/name: cronbird
     policyTypes:
       - Ingress
     ingress:
       - from:
         - namespaceSelector:
             matchLabels:
               name: monitoring  # Allow Prometheus
   ```

## Requirements

- Kubernetes 1.20+
- Helm 3.0+

## License

This chart follows the same license as the cronbird project.
