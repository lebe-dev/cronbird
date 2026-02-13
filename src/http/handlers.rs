use crate::domain::{CallbackStore, Identity};
use crate::http::errors::ApiError;
use crate::http::metrics_format::{JsonMetricsResponse, format_prometheus};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use serde::Deserialize;
use serde_json::json;

/// Query parameters for format selection.
#[derive(Debug, Deserialize)]
pub struct FormatQuery {
    format: Option<String>,
}

/// Health check handler.
/// GET /health
pub async fn health() -> Json<serde_json::Value> {
    Json(json!({
        "status": "ok"
    }))
}

/// Callback handler - records a callback for the given identity.
/// POST /callback/{identity}
pub async fn callback<S: CallbackStore>(
    State(store): State<S>,
    Path(identity_str): Path<String>,
) -> Result<StatusCode, ApiError> {
    let identity = Identity::new(identity_str)?;

    store.record_callback(&identity).await?;

    tracing::debug!("Recorded callback for identity: {}", identity);

    Ok(StatusCode::NO_CONTENT)
}

/// Metrics handler - returns all metrics.
/// GET /metrics
/// Supports content negotiation via Accept header or ?format=json query parameter.
pub async fn metrics<S: CallbackStore>(
    State(store): State<S>,
    headers: HeaderMap,
    Query(params): Query<FormatQuery>,
) -> Result<Response, ApiError> {
    let records = store
        .get_all_records()
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?;

    // Determine format from query param or Accept header
    let format = determine_format(&headers, params.format.as_deref());

    match format {
        MetricsFormat::Json => {
            let response = JsonMetricsResponse::from_records(&records);
            Ok(Json(response).into_response())
        }
        MetricsFormat::Prometheus => {
            let text = format_prometheus(&records);
            Ok((
                [("Content-Type", "text/plain; version=0.0.4; charset=utf-8")],
                text,
            )
                .into_response())
        }
    }
}

/// Metrics handler for a specific identity.
/// GET /metrics/{identity}
pub async fn metrics_single<S: CallbackStore>(
    State(store): State<S>,
    Path(identity_str): Path<String>,
    headers: HeaderMap,
    Query(params): Query<FormatQuery>,
) -> Result<Response, ApiError> {
    let identity = Identity::new(identity_str)?;

    let record = store
        .get_record(&identity)
        .await
        .map_err(|e| ApiError::InternalError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound(format!("Identity not found: {}", identity)))?;

    let format = determine_format(&headers, params.format.as_deref());

    match format {
        MetricsFormat::Json => {
            let metrics = vec![(identity, record)];
            let response = JsonMetricsResponse::from_records(&metrics);
            Ok(Json(response).into_response())
        }
        MetricsFormat::Prometheus => {
            let metrics = vec![(identity, record)];
            let text = format_prometheus(&metrics);
            Ok((
                [("Content-Type", "text/plain; version=0.0.4; charset=utf-8")],
                text,
            )
                .into_response())
        }
    }
}

/// Metrics format enum.
#[derive(Debug, Clone, Copy)]
enum MetricsFormat {
    Json,
    Prometheus,
}

/// Determines the metrics format from headers and query parameters.
/// Query parameter takes precedence over Accept header.
/// Defaults to Prometheus format.
fn determine_format(headers: &HeaderMap, format_param: Option<&str>) -> MetricsFormat {
    if let Some(format) = format_param
        && format.eq_ignore_ascii_case("json")
    {
        return MetricsFormat::Json;
    }

    if let Some(accept) = headers.get("accept")
        && let Ok(accept_str) = accept.to_str()
        && accept_str.contains("application/json")
    {
        return MetricsFormat::Json;
    }

    MetricsFormat::Prometheus
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::InMemoryStore;
    use axum::http::HeaderValue;

    #[tokio::test]
    async fn test_health() {
        let response = health().await;
        assert_eq!(response.0["status"], "ok");
    }

    #[tokio::test]
    async fn test_callback() {
        let store = InMemoryStore::new(None);
        let identity = "test-job".to_string();

        let result = callback(State(store.clone()), Path(identity.clone())).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), StatusCode::NO_CONTENT);

        // Verify record was created
        let identity_obj = Identity::new(identity).unwrap();
        let record = store.get_record(&identity_obj).await.unwrap();
        assert!(record.is_some());
    }

    #[tokio::test]
    async fn test_callback_invalid_identity() {
        let store = InMemoryStore::new(None);
        let identity = "invalid/identity".to_string();

        let result = callback(State(store), Path(identity)).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_determine_format_json_query() {
        let headers = HeaderMap::new();
        let format = determine_format(&headers, Some("json"));
        assert!(matches!(format, MetricsFormat::Json));
    }

    #[tokio::test]
    async fn test_determine_format_json_header() {
        let mut headers = HeaderMap::new();
        headers.insert("accept", HeaderValue::from_static("application/json"));
        let format = determine_format(&headers, None);
        assert!(matches!(format, MetricsFormat::Json));
    }

    #[tokio::test]
    async fn test_determine_format_default() {
        let headers = HeaderMap::new();
        let format = determine_format(&headers, None);
        assert!(matches!(format, MetricsFormat::Prometheus));
    }
}
