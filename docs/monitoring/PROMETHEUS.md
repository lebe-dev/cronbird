# Prometheus

Scrape Configuration:

```yaml
scrape_configs:
  - job_name: 'cronbird'
    static_configs:
      - targets: ['localhost:8080']
```

Examples:

```yaml
groups:
  - name: cronbird
    interval: 30s
    rules:
      # Alert when a cron job hasn't sent a callback in 2 hours
      - alert: CronJobMissed
        expr: time() - cronbird_last_callback_timestamp_seconds > 7200
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Cron job {{ $labels.identity }} missed callback"
          description: "No callback from {{ $labels.identity }} for {{ $value | humanizeDuration }}"

      # Alert when a cron job hasn't sent a callback in 24 hours (critical)
      - alert: CronJobMissedCritical
        expr: time() - cronbird_last_callback_timestamp_seconds > 86400
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "Cron job {{ $labels.identity }} not responding"
          description: "No callback from {{ $labels.identity }} for {{ $value | humanizeDuration }}"

      # Alert for specific critical jobs with tighter SLA (e.g., 1 hour for backups)
      - alert: BackupJobMissed
        expr: |
          time() - cronbird_last_callback_timestamp_seconds{identity=~".*backup.*"} > 3600
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "Backup job {{ $labels.identity }} overdue"
          description: "No callback from backup job {{ $labels.identity }} for {{ $value | humanizeDuration }}"

      # Example: Daily job should callback within 25 hours
      - alert: DailyJobMissed
        expr: |
          time() - cronbird_last_callback_timestamp_seconds{identity=~"daily-.*"} > 90000
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "Daily job {{ $labels.identity }} missed"
          description: "Daily job {{ $labels.identity }} hasn't run in over 25 hours"
```
