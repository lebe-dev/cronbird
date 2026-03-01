![cronbird](logo.png)

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
[![Build Status](https://img.shields.io/badge/build-passing-brightgreen)]()
[![License](https://img.shields.io/badge/license-MIT-blue)]()

**cronbird** is a lightweight HTTP service that monitors cron job execution through callback pings and exports metrics for Prometheus/VictoriaMetrics.

When a cron job completes, it sends a POST to cronbird. If a callback isn't received within the expected timeframe, your monitoring system raises an alert.

## Features

- Single binary, no external dependencies (no database, no Redis)
- ~3 MB memory, ~12.5 MB Docker image
- Prometheus text exposition format + JSON metrics
- Optional bearer token authentication
- Periodic state snapshots to JSON file
- Graceful shutdown, health checks, structured logging

## Quick Start

```bash
cp .env.example .env
# edit .env
docker compose up -d
```

Send a callback from your cron job:

```bash
pg_dump mydb > /backup/mydb.sql && \
  curl -X POST http://cronbird:8080/callback/daily-backup
```

Check metrics:

```bash
curl http://cronbird:8080/metrics
```

Set up an alert in Prometheus:

```yaml
- alert: CronJobMissed
  expr: time() - cronbird_last_callback_timestamp_seconds > 7200
  for: 5m
  annotations:
    summary: "Cron job {{ $labels.identity }} missed callback"
```

## Documentation

| Topic | Link |
|-------|------|
| Installation (Docker) | [docs/installation/DOCKER.md](docs/installation/DOCKER.md) |
| Installation (Kubernetes) | [docs/installation/KUBERNETES.md](docs/installation/KUBERNETES.md) |
| Installation (from source) | [docs/installation/SOURCE.md](docs/installation/SOURCE.md) |
| Configuration | [.env.example](.env.example) |
| API | [docs/API.md](docs/API.md) |
| Monitoring & Alerts | [docs/monitoring/PROMETHEUS.md](docs/monitoring/PROMETHEUS.md) |
| Security | [docs/security/SECURITY.md](docs/security/SECURITY.md) |
| Troubleshooting | [docs/TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md) |
| Development | [DEV.md](DEV.md) |

## License

MIT
