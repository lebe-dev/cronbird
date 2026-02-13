# Installation with Docker

This guide explains how to deploy **cronbird** using Docker and Docker Compose.

## Prerequisites

- [Docker](https://docs.docker.com/get-docker/) installed on your system.
- [Docker Compose](https://docs.docker.com/compose/install/) (optional, but recommended).

## Using Docker Compose (Recommended)

Docker Compose is the easiest way to get cronbird up and running with persistence and proper configuration.

1. **Create a directory for your deployment:**
   ```bash
   mkdir cronbird && cd cronbird
   ```

2. **Download the `docker-compose.yml` and `.env.example` files:**
   ```bash
   curl -L https://raw.githubusercontent.com/lebe-dev/cronbird/main/docker-compose.yml -o docker-compose.yml
   curl -L https://raw.githubusercontent.com/lebe-dev/cronbird/main/.env.example -o .env
   ```

3. **Configure your environment:**
   Edit the `.env` file to suit your needs. See the [Configuration](#configuration) section below.

4. **Start the service:**
   ```bash
   docker compose up -d
   ```

The service will be available at `http://localhost:18080`.

## Using Docker CLI

If you prefer to run a single container without Compose:

```bash
docker run -d \
  --name cronbird \
  -p 8080:8080 \
  -v $(pwd)/data:/data \
  -e CRONBIRD_ALLOW_DYNAMIC=true \
  -e CRONBIRD_PERSIST_PATH=/data/state.json \
  tinyops/cronbird:0.1.0
```

## Configuration

| Variable | Default | Description |
|----------|---------|-------------|
| `CRONBIRD_LISTEN` | `0.0.0.0:8080` | Bind address inside container |
| `CRONBIRD_IDENTITIES` | `""` | Comma-separated allowed job IDs |
| `CRONBIRD_ALLOW_DYNAMIC` | `false` | If true, any ID is accepted |
| `CRONBIRD_AUTH_TOKEN` | `""` | Bearer token (recommended) |
| `CRONBIRD_PERSIST_PATH` | `./cronbird-state.json` | Path to save state |
| `CRONBIRD_PERSIST_INTERVAL` | `60` | Save interval in seconds |
| `CRONBIRD_LOG_LEVEL` | `info` | logging level |

## Data Persistence

By default, cronbird stores its state in memory and periodically flushes it to a JSON file. To ensure data survives container restarts, you **must** mount a volume to the location specified in `CRONBIRD_PERSIST_PATH`.

In the Docker Compose example, we use:
```yaml
volumes:
  - ./data:/data
```
And set `CRONBIRD_PERSIST_PATH=/data/cronbird-state.json`.

## Health Checks

The Docker image includes a health check that monitors the `/health` endpoint. You can check the status using:

```bash
docker ps
```

You should see `(healthy)` next to the status.

## Updating

To update to the latest version:

```bash
docker compose pull
docker compose up -d
```
