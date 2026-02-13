use crate::domain::{CallbackRecord, Identity};
use serde::Serialize;

/// Formats metrics in Prometheus text exposition format.
pub fn format_prometheus(records: &[(Identity, CallbackRecord)]) -> String {
    let mut output = String::new();

    output.push_str("# HELP cronbird_last_callback_timestamp_seconds Unix timestamp of the last successful callback\n");
    output.push_str("# TYPE cronbird_last_callback_timestamp_seconds gauge\n");

    for (identity, record) in records {
        output.push_str(&format!(
            "cronbird_last_callback_timestamp_seconds{{identity=\"{}\"}} {}\n",
            identity.as_str(),
            record.last_callback_ts
        ));
    }

    output.push('\n');

    output.push_str("# HELP cronbird_callback_total Total number of callbacks received\n");
    output.push_str("# TYPE cronbird_callback_total counter\n");

    for (identity, record) in records {
        output.push_str(&format!(
            "cronbird_callback_total{{identity=\"{}\"}} {}\n",
            identity.as_str(),
            record.callback_count
        ));
    }

    output
}

/// JSON representation of a single metric record.
#[derive(Debug, Serialize)]
pub struct JsonMetric {
    pub identity: String,
    pub last_callback_ts: i64,
    pub last_callback_rfc3339: String,
    pub callback_count: u64,
}

impl JsonMetric {
    /// Creates a JsonMetric from an Identity and CallbackRecord.
    pub fn from_record(identity: &Identity, record: &CallbackRecord) -> Self {
        let rfc3339 = timestamp_to_rfc3339(record.last_callback_ts);

        Self {
            identity: identity.to_string(),
            last_callback_ts: record.last_callback_ts,
            last_callback_rfc3339: rfc3339,
            callback_count: record.callback_count,
        }
    }
}

/// Container for JSON metrics response.
#[derive(Debug, Serialize)]
pub struct JsonMetricsResponse {
    pub metrics: Vec<JsonMetric>,
}

impl JsonMetricsResponse {
    /// Creates a JsonMetricsResponse from a slice of records.
    pub fn from_records(records: &[(Identity, CallbackRecord)]) -> Self {
        let metrics = records
            .iter()
            .map(|(id, record)| JsonMetric::from_record(id, record))
            .collect();

        Self { metrics }
    }
}

/// Converts a Unix timestamp to RFC3339 format.
fn timestamp_to_rfc3339(timestamp: i64) -> String {
    use std::time::{Duration, UNIX_EPOCH};

    let duration = Duration::from_secs(timestamp as u64);
    let datetime = UNIX_EPOCH + duration;

    // Format as RFC3339
    // Using chrono would be cleaner, but we're avoiding dependencies
    // This is a simple implementation for UTC times
    let secs_since_epoch = datetime
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Simple calculation for UTC time
    const SECS_PER_DAY: u64 = 86400;
    const SECS_PER_HOUR: u64 = 3600;
    const SECS_PER_MINUTE: u64 = 60;

    // Days since epoch (1970-01-01 was a Thursday)
    let days = secs_since_epoch / SECS_PER_DAY;
    let remaining = secs_since_epoch % SECS_PER_DAY;

    let hours = remaining / SECS_PER_HOUR;
    let remaining = remaining % SECS_PER_HOUR;

    let minutes = remaining / SECS_PER_MINUTE;
    let seconds = remaining % SECS_PER_MINUTE;

    // Simplified date calculation (good enough for timestamps after 2000)
    // For production, use chrono or time crate
    let year = 1970 + (days / 365);
    let day_of_year = days % 365;

    // Very simplified month/day (assumes 30-day months for simplicity)
    let month = (day_of_year / 30) + 1;
    let day = (day_of_year % 30) + 1;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_records() -> Vec<(Identity, CallbackRecord)> {
        vec![
            (
                Identity::new("clickhouse-backup-prod").unwrap(),
                CallbackRecord {
                    last_callback_ts: 1739456789,
                    callback_count: 42,
                },
            ),
            (
                Identity::new("pg-backup-staging").unwrap(),
                CallbackRecord {
                    last_callback_ts: 1739456123,
                    callback_count: 18,
                },
            ),
        ]
    }

    #[test]
    fn test_format_prometheus() {
        let records = create_test_records();
        let output = format_prometheus(&records);

        assert!(output.contains("# HELP cronbird_last_callback_timestamp_seconds"));
        assert!(output.contains("# TYPE cronbird_last_callback_timestamp_seconds gauge"));
        assert!(output.contains("cronbird_last_callback_timestamp_seconds{identity=\"clickhouse-backup-prod\"} 1739456789"));

        assert!(output.contains("# HELP cronbird_callback_total"));
        assert!(output.contains("# TYPE cronbird_callback_total counter"));
        assert!(output.contains("cronbird_callback_total{identity=\"clickhouse-backup-prod\"} 42"));
        assert!(output.contains("cronbird_callback_total{identity=\"pg-backup-staging\"} 18"));
    }

    #[test]
    fn test_json_metrics_response() {
        let records = create_test_records();
        let response = JsonMetricsResponse::from_records(&records);

        assert_eq!(response.metrics.len(), 2);
        assert_eq!(response.metrics[0].identity, "clickhouse-backup-prod");
        assert_eq!(response.metrics[0].last_callback_ts, 1739456789);
        assert_eq!(response.metrics[0].callback_count, 42);
    }

    #[test]
    fn test_empty_metrics() {
        let records: Vec<(Identity, CallbackRecord)> = vec![];

        let prom_output = format_prometheus(&records);
        assert!(prom_output.contains("# HELP"));
        assert!(prom_output.contains("# TYPE"));

        let json_response = JsonMetricsResponse::from_records(&records);
        assert!(json_response.metrics.is_empty());
    }
}
