use crate::domain::{CallbackRecord, Identity};
use serde::Serialize;

/// Escapes special characters in Prometheus label values.
/// Defence-in-depth: escapes `\` and `"` even though Identity validation
/// prevents these characters, in case validation is relaxed in the future.
fn escape_label_value(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Formats metrics in Prometheus text exposition format.
pub fn format_prometheus(records: &[(Identity, CallbackRecord)]) -> String {
    let mut output = String::new();

    output.push_str("# HELP cronbird_last_callback_timestamp_seconds Unix timestamp of the last successful callback\n");
    output.push_str("# TYPE cronbird_last_callback_timestamp_seconds gauge\n");

    for (identity, record) in records {
        let escaped_identity = escape_label_value(identity.as_str());
        output.push_str(&format!(
            "cronbird_last_callback_timestamp_seconds{{identity=\"{}\"}} {}\n",
            escaped_identity, record.last_callback_ts
        ));
    }

    output.push('\n');

    output.push_str("# HELP cronbird_callback_total Total number of callbacks received\n");
    output.push_str("# TYPE cronbird_callback_total counter\n");

    for (identity, record) in records {
        let escaped_identity = escape_label_value(identity.as_str());
        output.push_str(&format!(
            "cronbird_callback_total{{identity=\"{}\"}} {}\n",
            escaped_identity, record.callback_count
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

/// Converts a Unix timestamp to RFC3339 format (UTC).
///
/// Uses Howard Hinnant's civil_from_days algorithm for correct Gregorian calendar
/// arithmetic, including leap years. Handles negative timestamps safely.
fn timestamp_to_rfc3339(timestamp: i64) -> String {
    const SECS_PER_DAY: i64 = 86400;
    const SECS_PER_HOUR: u64 = 3600;
    const SECS_PER_MINUTE: u64 = 60;

    // div_euclid / rem_euclid handle negative timestamps correctly.
    let days = timestamp.div_euclid(SECS_PER_DAY);
    let day_secs = timestamp.rem_euclid(SECS_PER_DAY) as u64;

    let hours = day_secs / SECS_PER_HOUR;
    let minutes = (day_secs % SECS_PER_HOUR) / SECS_PER_MINUTE;
    let seconds = day_secs % SECS_PER_MINUTE;

    let (year, month, day) = civil_from_days(days);

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

/// Converts days-since-Unix-epoch to a proleptic Gregorian (year, month, day).
///
/// Algorithm: Howard Hinnant, "date_algorithms" (public domain).
/// <https://howardhinnant.github.io/date_algorithms.html>
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468; // shift epoch from 1970-01-01 to 0000-03-01
    let era = (if z >= 0 { z } else { z - 146_096 }) / 146_097;
    let doe = (z - era * 146_097) as u32; // day of era [0, 146096]
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365; // year of era [0, 399]
    let y = yoe as i32 + era as i32 * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // day of year [0, 365]
    let mp = (5 * doy + 2) / 153; // month piece [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // day [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // month [1, 12]
    let year = y + if m <= 2 { 1 } else { 0 };
    (year, m, d)
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
    fn test_timestamp_to_rfc3339() {
        // Unix epoch
        assert_eq!(timestamp_to_rfc3339(0), "1970-01-01T00:00:00Z");
        // 2026-03-01T00:00:00Z = 20513 days * 86400
        assert_eq!(timestamp_to_rfc3339(1_772_323_200), "2026-03-01T00:00:00Z");
        // Leap day: 2024-02-29T12:00:00Z = 19782 days * 86400 + 43200
        assert_eq!(timestamp_to_rfc3339(1_709_208_000), "2024-02-29T12:00:00Z");
        // Negative: 1969-12-31T23:59:59Z = -1
        assert_eq!(timestamp_to_rfc3339(-1), "1969-12-31T23:59:59Z");
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

    #[test]
    fn test_label_escaping() {
        // Test escape_label_value with backslash and quote characters
        assert_eq!(escape_label_value("normal"), "normal");
        assert_eq!(escape_label_value("with\\backslash"), "with\\\\backslash");
        assert_eq!(escape_label_value("with\"quote"), "with\\\"quote");
        assert_eq!(escape_label_value("both\\\"chars"), "both\\\\\\\"chars");
    }
}
