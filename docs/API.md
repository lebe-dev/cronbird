# API

### `POST /callback/{identity}`

Records a callback for the specified identity.

**Authentication:** Required if `CRONBIRD_AUTH_TOKEN` is set.

**Example:**
```bash
curl -X POST \
  -H "Authorization: Bearer your-secret-token" \
  http://localhost:8080/callback/clickhouse-backup-prod
```

**Responses:**
- `204 No Content` - Success
- `401 Unauthorized` - Invalid/missing token
- `403 Forbidden` - Identity not in allow-list

### `GET /metrics`

Returns metrics for all identities.

**Content Negotiation:**
- `Accept: application/json` or `?format=json` → JSON
- Default → Prometheus text exposition format

**Prometheus Format:**
```
# HELP cronbird_last_callback_timestamp_seconds Unix timestamp of the last successful callback
# TYPE cronbird_last_callback_timestamp_seconds gauge
cronbird_last_callback_timestamp_seconds{identity="clickhouse-backup-prod"} 1739456789

# HELP cronbird_callback_total Total number of callbacks received
# TYPE cronbird_callback_total counter
cronbird_callback_total{identity="clickhouse-backup-prod"} 42
```

**JSON Format:**
```json
{
  "metrics": [
    {
      "identity": "clickhouse-backup-prod",
      "last_callback_ts": 1739456789,
      "last_callback_rfc3339": "2025-02-13T15:33:09Z",
      "callback_count": 42
    }
  ]
}
```

### `GET /metrics/{identity}`

Returns metrics for a specific identity. Same content negotiation as `/metrics`.

**Responses:**
- `200 OK` - Metrics returned
- `404 Not Found` - Identity not found

### `GET /health`

Health check endpoint.

**Response:**
```json
{
  "status": "ok"
}
```
