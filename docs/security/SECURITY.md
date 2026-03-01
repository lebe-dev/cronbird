# cronbird Security

This document outlines the security features, threat model, and best practices for deploying cronbird.

## Overview

cronbird is a lightweight HTTP service for monitoring cron job execution. It does not handle sensitive data beyond callback metadata (timestamps and counts) but implements security controls to prevent unauthorized access and ensure data integrity.

## Authentication & Authorization

### Bearer Token Authentication

cronbird supports optional Bearer token authentication to restrict who can submit callback data:

```
Authorization: Bearer <token>
```

**Security Properties:**
- **Constant-time Comparison**: Token validation uses constant-time string comparison (`secure_compare`) to prevent timing attacks. All bytes are compared regardless of differences, eliminating information leakage about token correctness.
- **Optional Mode**: If `CRONBIRD_AUTH_TOKEN` is not set or empty, authentication is disabled (no-op passthrough). This is suitable for internal networks but **not recommended for production**.
- **No Rate Limiting**: The service does not implement rate limiting on the `/callback/*` endpoints. Consider placing cronbird behind a rate-limiting reverse proxy or API gateway in production.

**Configuration:**
```bash
# Enable authentication
CRONBIRD_AUTH_TOKEN=your-secret-bearer-token

# Disable authentication (default)
CRONBIRD_AUTH_TOKEN=
```

**Recommended Practice:**
- Always set a strong, random token for production deployments.
- Rotate tokens periodically using your reverse proxy or API gateway.
- Use HTTPS to prevent token interception in transit (cronbird serves HTTP; TLS must be terminated upstream).

## Input Validation

### Identity Validation

All cron job identities are validated to prevent injection attacks and enforce consistent naming:

**Rules:**
- Characters allowed: `[a-zA-Z0-9_-]` (alphanumeric, underscore, dash)
- Length: 1–128 characters
- Empty identities are rejected
- Identities with special characters (spaces, `/`, `@`, etc.) are rejected

**Example Valid Identities:**
- `clickhouse-backup-prod`
- `pg_backup_staging`
- `backup123`

**Example Invalid Identities:**
- `backup/prod` (forbidden character: `/`)
- `backup@example` (forbidden character: `@`)
- `backup prod` (forbidden character: space)

**Protection Against Attacks:**
- **Path Traversal**: Restricted character set prevents `../` or other path traversal attempts.
- **JSON/XML Injection**: Identities are used only as HashMap keys and Prometheus label values; the validation ensures they cannot inject control characters.
- **Command Injection**: If identities are used in downstream scripts or commands, the restricted character set provides defense-in-depth.

## Container Security Reports

- [trivy-scan-report.txt](trivy-scan-report.txt).
- [dockle-scan-report.txt](dockle-scan-report.txt).

### Runtime Isolation

cronbird runs in a non-root container with the following hardening:

```dockerfile
# Non-root user
USER cronbird

# Minimal base image
FROM alpine:3.23.3

# Restricted permissions
RUN chmod 700 /app
RUN chown -R cronbird: /app
```

**Security Benefits:**
- **Non-root Execution**: Prevents privilege escalation if the application is compromised.
- **Minimal Base Image**: Alpine Linux (1.4 MB) reduces attack surface compared to full Linux distributions.
- **Restricted Home Directory**: The `cronbird` user's home directory is `/app` with 700 permissions (read/write/execute for owner only).

## Data Persistence

### Atomic Write Safety

State snapshots are persisted atomically to prevent corruption:

```rust
// Write to temporary file
let tmp_path = path.with_extension("tmp");
tokio::fs::write(&tmp_path, json).await?;

// Atomic rename
tokio::fs::rename(&tmp_path, &path).await?;
```

**Protection:**
- Process crash or power loss during write does not corrupt the state file.
- Old state remains intact if the write fails partway through.

### State Isolation

- State is stored in a single JSON file (default: `./cronbird-state.json`).
- File permissions are inherited from the container umask; ensure the file is readable only by the `cronbird` user.
- The in-memory store uses `Arc<RwLock<HashMap>>` for safe concurrent access.

**File Permissions Recommendation:**
```bash
# After initialization, set restrictive permissions
chmod 600 cronbird-state.json
chown cronbird:cronbird cronbird-state.json
```

### Persistence Interval

State is persisted at regular intervals (default: 60 seconds):

```bash
CRONBIRD_PERSIST_INTERVAL=60  # Seconds
```

**Data Loss Window:** If the service crashes between persistence intervals, you can lose up to `CRONBIRD_PERSIST_INTERVAL` seconds of callback data. For high-frequency job monitoring, reduce the interval at the cost of increased disk I/O.

## Network Security

### Security Headers

All HTTP responses include security headers to prevent common attacks:

| Header | Value | Purpose |
|--------|-------|---------|
| `X-Content-Type-Options` | `nosniff` | Prevents MIME sniffing attacks; forces the browser to respect the Content-Type header |
| `X-Frame-Options` | `DENY` | Prevents clickjacking by disallowing the response from being framed |
| `Cache-Control` | `no-store` | Prevents caching of metrics and sensitive responses in browsers and proxies |

These headers are applied globally to all endpoints (health, metrics, callback) to ensure consistent security posture.

### HTTP Protocol

cronbird serves HTTP (not HTTPS). This is intentional because:

1. **TLS Termination**: Deployed behind a reverse proxy (nginx, Traefik, AWS ALB) that handles TLS.
2. **Internal Networks**: Often used on private networks where encryption is less critical.
3. **Reduced Complexity**: Avoids managing certificates in the application.

**Deployment Pattern:**
```
Internet (HTTPS) → Reverse Proxy (TLS termination) → cronbird (HTTP on localhost or private network)
```

### Health Check Endpoint

The `/health` endpoint provides a simple liveness probe:

```bash
GET /health
# Response: 200 OK with "OK"
```

This is used by container orchestrators for health monitoring and does not require authentication.

### Metrics Endpoints

- `GET /metrics` - Prometheus format (no authentication required)
- `GET /metrics/json` - JSON format (no authentication required)

**Security Note:** The `/metrics` endpoints are intentionally unauthenticated to allow Prometheus scrapers on private networks to collect data without managing secrets. If you expose `/metrics` to untrusted networks, consider placing it behind an authentication proxy.

## Callback Endpoints

- `POST /callback/{identity}` - Requires `CRONBIRD_AUTH_TOKEN` if configured
- `GET /callback/{identity}` - Requires `CRONBIRD_AUTH_TOKEN` if configured

Both endpoints validate the identity before processing. Invalid identities are rejected with `400 Bad Request`.

## Operational Security

### Configuration Management

All configuration is via environment variables; secrets are never hardcoded:

```bash
# .env file (use a secrets manager in production)
CRONBIRD_AUTH_TOKEN=your-secret-token
CRONBIRD_PERSIST_PATH=/secure/path/cronbird-state.json
```

**Best Practices:**
- Use a secrets manager (Vault, AWS Secrets Manager, Kubernetes Secrets) instead of `.env` files.
- Never commit `.env` files or secrets to version control.
- Rotate `CRONBIRD_AUTH_TOKEN` periodically.

### Graceful Shutdown

On termination signals (`SIGINT`, `SIGTERM`), the service:

1. Stops accepting new requests.
2. Waits for in-flight requests to complete.
3. Performs a final persistence flush.

This ensures no callback data is lost during deployment or scaling operations.

### Logging & Observability

cronbird uses structured logging with `tracing`:

```bash
CRONBIRD_LOG_LEVEL=debug  # debug, info, warn, error
```

**Security Considerations:**
- Log level `debug` may expose internal state; use `info` or `warn` in production.
- Logs are written to stdout/stderr and should be aggregated by your logging infrastructure.
- The Bearer token itself is never logged.
- Callback data (identity, timestamp, count) is non-sensitive and logged at debug level.

## Threat Model

### Attack Vectors & Mitigations

| Threat | Vector | Mitigation |
|--------|--------|-----------|
| **Unauthorized Callbacks** | Attacker submits callback for job they don't own | Bearer token authentication + whitelist identities |
| **Timing Attack** | Attacker infers token via response time | Constant-time token comparison |
| **Token Interception** | Attacker intercepts Bearer token over HTTP | HTTPS at reverse proxy + short-lived tokens |
| **Invalid Identities** | Attacker submits special characters to break downstream tools | Strict identity validation (alphanumeric, dash, underscore) |
| **State Corruption** | Process crash during file write | Atomic writes (temp file → rename) |
| **Privilege Escalation** | Attacker exploits app to gain root access | Non-root container user |
| **Denial of Service** | Attacker floods `/callback/*` with requests | No built-in rate limiting; use reverse proxy throttling |
| **State Exfiltration** | Attacker reads `cronbird-state.json` | File permissions (600) + mount on encrypted volume |
| **Clickjacking** | Attacker frames the app in a malicious page | X-Frame-Options: DENY header |
| **MIME Sniffing** | Browser interprets response as different content type | X-Content-Type-Options: nosniff header |
| **Cache Poisoning** | Cached metrics serve stale data to multiple users | Cache-Control: no-store header |

### Out-of-Scope Threats

The following threats are **outside cronbird's threat model** and should be addressed at the deployment level:

- **DDoS Attacks**: Mitigated by reverse proxy or CDN.
- **Infrastructure Compromise**: Mitigated by container orchestration (k8s RBAC, network policies).
- **Stolen Credentials**: Mitigated by secret rotation and audit logging.
- **Malicious Reverse Proxy**: Assume the reverse proxy is trustworthy.

## Security Considerations & Limitations

### Known Limitations

1. **No Encryption at Rest**: State file is stored in plaintext JSON. For sensitive environments, encrypt the volume or use a remote store.
2. **No Audit Trail**: Callback submissions are logged but not persisted as an audit trail. For compliance, pipe logs to an audit log store.
3. **No Rate Limiting**: The service does not rate-limit callback submissions. High-frequency attacks can create state churn. Use a reverse proxy to enforce per-IP or per-token limits.
4. **Static Identity Whitelist**: Identities are configured at startup. Changing the list requires a restart (no hot-reload).
5. **Single-Instance**: State is in-memory on a single instance. Multi-instance deployments must use an external store (implement the `CallbackStore` trait for Redis, PostgreSQL, etc.).

### Recommended Deployment Practices

#### Development
```bash
# Auth disabled, dynamic identities allowed, in-memory only
CRONBIRD_ALLOW_DYNAMIC=true
CRONBIRD_AUTH_TOKEN=
CRONBIRD_PERSIST_PATH=./cronbird-state.json
```

#### Staging
```bash
# Predefined identities, authentication enabled
CRONBIRD_ALLOW_DYNAMIC=false
CRONBIRD_IDENTITIES=job1,job2,job3
CRONBIRD_AUTH_TOKEN=your-staging-token
CRONBIRD_PERSIST_PATH=/data/cronbird-state.json
```

#### Production
```bash
# Strict configuration, strong token, encrypted volume
CRONBIRD_ALLOW_DYNAMIC=false
CRONBIRD_IDENTITIES=job1,job2,job3
CRONBIRD_AUTH_TOKEN=$(tr -dc 'A-Za-z0-9' < /dev/urandom | head -c 32)
CRONBIRD_PERSIST_PATH=/secure/cronbird-state.json
CRONBIRD_PERSIST_INTERVAL=30
CRONBIRD_LOG_LEVEL=info
```

#### Network Isolation
```yaml
# Kubernetes example with network policy
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: cronbird
spec:
  podSelector:
    app: cronbird
  policyTypes:
  - Ingress
  ingress:
  - from:
    - podSelector:
        app: ingress-controller
    ports:
    - protocol: TCP
      port: 8080
```

### Monitoring & Alerting

Monitor the following metrics for security incidents:

- **Failed Authentication**: High rate of `401 Unauthorized` responses.
- **Invalid Identities**: High rate of `400 Bad Request` for `/callback/*`.
- **Persistence Errors**: Log errors during state persistence indicate disk issues.
- **Graceful Shutdown**: Verify final persist completes before container termination.

## Security Reporting

If you discover a security vulnerability in cronbird, please report it responsibly:

1. Do **not** open a public GitHub issue.
2. Email the maintainers with a detailed description and reproduction steps.
3. Allow 90 days for a fix before public disclosure.

## References

- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [CWE-208: Observable Timing Discrepancy](https://cwe.mitre.org/data/definitions/208.html) (timing attacks)
- [NIST Cybersecurity Framework](https://www.nist.gov/cyberframework)
- [Container Security Best Practices](https://docs.docker.com/engine/security/)
