use crate::domain::CallbackStore;
use crate::http::auth::AuthMiddleware;
use crate::http::handlers::{callback, health_route, metrics, metrics_single};
use axum::{
    Router,
    extract::Request,
    http::{HeaderValue, header},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
};
use tower_http::limit::RequestBodyLimitLayer;

/// Middleware to add security headers to all responses.
async fn add_security_headers(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(header::X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));

    response
}

/// Builds the HTTP router with all routes.
pub fn build_router<S: CallbackStore>(
    store: S,
    auth: AuthMiddleware,
    protect_metrics: bool,
) -> Router {
    let callback_routes = Router::new().route("/callback/{identity}", post(callback::<S>));

    let callback_routes = if auth.is_enabled() {
        let auth_clone = auth.clone();
        callback_routes.layer(middleware::from_fn(move |headers, request, next| {
            let auth = auth_clone.clone();
            async move { auth.handle(headers, request, next).await }
        }))
    } else {
        callback_routes
    };

    let metrics_routes = Router::new()
        .route("/metrics", get(metrics::<S>))
        .route("/metrics/{identity}", get(metrics_single::<S>));

    let metrics_routes = if protect_metrics && auth.is_enabled() {
        let auth_clone = auth.clone();
        metrics_routes.layer(middleware::from_fn(move |headers, request, next| {
            let auth = auth_clone.clone();
            async move { auth.handle(headers, request, next).await }
        }))
    } else {
        metrics_routes
    };

    let public_routes = Router::new().route("/health", get(health_route));

    Router::new()
        .merge(callback_routes)
        .merge(metrics_routes)
        .merge(public_routes)
        .with_state(store)
        .layer(RequestBodyLimitLayer::new(1024)) // 1 KB — callbacks have no body
        .layer(middleware::from_fn(add_security_headers))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::InMemoryStore;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use std::collections::HashSet;
    use tower::ServiceExt;

    fn store_with_identity(identity: &str) -> InMemoryStore {
        let mut allowed = HashSet::new();
        allowed.insert(identity.to_string());
        InMemoryStore::new(Some(allowed))
    }

    #[tokio::test]
    async fn test_metrics_public_by_default() {
        let store = store_with_identity("job1");
        let auth = AuthMiddleware::new(None);
        let app = build_router(store, auth, false);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_metrics_protected_requires_token() {
        let store = store_with_identity("job1");
        let auth = AuthMiddleware::new(Some("secret".to_string()));
        let app = build_router(store, auth, true);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_metrics_protected_with_valid_token() {
        let store = store_with_identity("job1");
        let auth = AuthMiddleware::new(Some("secret".to_string()));
        let app = build_router(store, auth, true);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .header("Authorization", "Bearer secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_metrics_unprotected_when_no_token() {
        // protect_metrics=true but no token configured → metrics remain public
        let store = store_with_identity("job1");
        let auth = AuthMiddleware::new(None);
        let app = build_router(store, auth, true);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_health_always_public() {
        let store = store_with_identity("job1");
        let auth = AuthMiddleware::new(Some("secret".to_string()));
        let app = build_router(store, auth, true);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_security_headers_present() {
        let store = store_with_identity("job1");
        let auth = AuthMiddleware::new(None);
        let app = build_router(store, auth, false);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let headers = response.headers();

        // Verify security headers are present
        assert_eq!(
            headers
                .get(header::X_CONTENT_TYPE_OPTIONS)
                .and_then(|h| h.to_str().ok()),
            Some("nosniff")
        );
        assert_eq!(
            headers
                .get(header::X_FRAME_OPTIONS)
                .and_then(|h| h.to_str().ok()),
            Some("DENY")
        );
        assert_eq!(
            headers
                .get(header::CACHE_CONTROL)
                .and_then(|h| h.to_str().ok()),
            Some("no-store")
        );
    }
}
