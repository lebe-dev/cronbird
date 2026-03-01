use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

/// HTTP API errors.
#[derive(Debug)]
pub enum ApiError {
    Unauthorized,
    Forbidden(String),
    NotFound,
    BadRequest(String),
    StoreFull,
    InternalError,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
            ApiError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            ApiError::NotFound => (StatusCode::NOT_FOUND, "Not found".to_string()),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            ApiError::StoreFull => (
                StatusCode::INSUFFICIENT_STORAGE,
                "Identity limit reached".to_string(),
            ),
            ApiError::InternalError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
        };

        let body = Json(json!({
            "error": message,
        }));

        (status, body).into_response()
    }
}

impl From<crate::domain::IdentityError> for ApiError {
    fn from(err: crate::domain::IdentityError) -> Self {
        ApiError::BadRequest(err.to_string())
    }
}

impl From<crate::domain::StoreError> for ApiError {
    fn from(err: crate::domain::StoreError) -> Self {
        match err {
            crate::domain::StoreError::IdentityNotAllowed(id) => {
                ApiError::Forbidden(format!("Identity not allowed: {}", id))
            }
            crate::domain::StoreError::StoreFull => ApiError::StoreFull,
            crate::domain::StoreError::Internal(msg) => {
                tracing::error!("Internal store error: {}", msg);
                ApiError::InternalError
            }
        }
    }
}
