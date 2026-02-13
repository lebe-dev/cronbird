![cronbird](logo.png)

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)]()
[![License](https://img.shields.io/badge/license-MIT-blue)]()

**cronbird** is a lightweight HTTP service that monitors cron job execution through callback pings and exports metrics for external monitoring systems like Prometheus and VictoriaMetrics.

When a cron job completes successfully, it sends a callback to cronbird. If a callback isn't received within the expected timeframe, your monitoring system raises an alert.

## Features

- **Lightweight** - Single binary, no external dependencies (no database, no Redis)
- **BLAZING FAST 🦀** (Rust)
- **Low memory footprint:**
  ```bash
  CONTAINER ID   NAME       CPU %     MEM USAGE / LIMIT     MEM %     NET I/O         BLOCK I/O       PIDS
  8e839850292d   cronbird   0.00%     2.664MiB / 7.807GiB   0.03%     1.13kB / 126B   131kB / 4.1kB   11
  ```
- **Tiny image size** - `12.5 MB`
- **Prometheus-compatible** - Native text exposition format + JSON metrics
- **Authentication** - Bearer token support for callback endpoints (optional)
- **State persistence** - Automatic periodic snapshots to JSON file
- **Identity management** - Predefined allow-list or dynamic mode
- **Production-ready** - Graceful shutdown, health checks, structured logging

## Quick Start

```bash
cp .env.example .env

# Edit .env file

# Run
docker compose up -d
```

Other options: [Kubernetes (Helm)](docs/installation/KUBERNETES.md), [Build from Source](docs/installation/SOURCE.md).

### Basic Usage

1. **Send a callback from your cron job:**

```bash
#!/bin/bash
pg_dump mydb > /backup/mydb.sql && \
curl -X POST http://somehost:8080/callback/daily-backup
```

**With authentication enabled:**

```bash
curl -X POST -H "Authorization: Bearer your-secret-token-here" http://somehost:8080/callback/daily-backup
```

2. **View metrics:**

```bash
# Prometheus format (default)
curl http://somehost:8080/metrics

# JSON format
curl -H "Accept: application/json" http://somehost:8080/metrics
# or
curl http://somehost:8080/metrics?format=json
```

3. **Configure Prometheus alerts:**

```yaml
- alert: CronJobMissed
  expr: time() - cronbird_last_callback_timestamp_seconds > 7200
  for: 5m
  annotations:
    summary: "Cron job {{ $labels.identity }} missed callback"
```

4. **Check metrics:**

```bash
# In another terminal, send a test callback
curl -X POST http://localhost:8080/callback/test-job

# Check metrics
curl http://localhost:8080/metrics

# You should see:
# cronbird_last_callback_timestamp_seconds{identity="test-job"} 1739456789
# cronbird_callback_total{identity="test-job"} 1
```

### Crontab Example

You can add the callback directly to your crontab:

```cron
# Every day at 2:00 AM
0 2 * * * /usr/local/bin/backup.sh && curl -X POST -H "Authorization: Bearer your-secret-token-here" http://localhost:8080/callback/daily-backup
```

## Configuration

All configuration is done through environment variables:

| Variable | Default | Description |
|----------|---------|-------------|
| `CRONBIRD_LISTEN` | `0.0.0.0:8080` | Bind address |
| `CRONBIRD_IDENTITIES` | `""` | Comma-separated predefined identities |
| `CRONBIRD_ALLOW_DYNAMIC` | `false` | Accept unknown identities |
| `CRONBIRD_AUTH_TOKEN` | `""` | Bearer token (empty = disabled) |
| `CRONBIRD_PERSIST_PATH` | `./cronbird-state.json` | State file path |
| `CRONBIRD_PERSIST_INTERVAL` | `60` | Seconds between persists |
| `CRONBIRD_LOG_LEVEL` | `info` | Log level (debug, info, warn, error) |

### Example .env file

```bash
CRONBIRD_LISTEN=0.0.0.0:8080
CRONBIRD_IDENTITIES=clickhouse-backup-prod,pg-backup-staging
CRONBIRD_ALLOW_DYNAMIC=false
CRONBIRD_AUTH_TOKEN=your-secret-token-here
CRONBIRD_PERSIST_PATH=/var/lib/cronbird/state.json
CRONBIRD_PERSIST_INTERVAL=60
CRONBIRD_LOG_LEVEL=info
```

## Monitoring Setup

- [Prometheus/VictoriaMetrics](docs/monitoring/PROMETHEUS.md)

## Development

See [DEV.md](DEV.md).

## Use Cases

### Backup Jobs

Monitor critical backup operations:

```bash
#!/bin/bash
clickhouse-backup create && \
clickhouse-backup upload && \
curl -X POST -H "Authorization: Bearer $CRONBIRD_TOKEN" http://cronbird/callback/clickhouse-backup-prod
```

### Data Pipelines

Track ETL job completion:

```bash
#!/bin/bash
python run_etl.py && \
curl -X POST http://cronbird/callback/daily-etl-pipeline
```

## Troubleshooting

See [TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md).

## Limitations

- **Single instance** - In-memory state isn't shared between replicas
- **Persist interval** - May lose up to N seconds of data on crash
- **No TTL** - Records persist indefinitely (use monitoring for cleanup)
- **No history** - Only last callback timestamp is stored

## Contributing

Contributions welcome! Please open an issue or PR.

## License

MIT License - see LICENSE file for details.
