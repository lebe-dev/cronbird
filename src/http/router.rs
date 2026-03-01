use crate::domain::CallbackStore;
use crate::http::auth::AuthMiddleware;
use crate::http::handlers::{callback, health, metrics, metrics_single};
use axum::{
    Router, middleware,
    routing::{get, post},
};
use tower_http::limit::RequestBodyLimitLayer;

/// Builds the HTTP router with all routes.
pub fn build_router<S: CallbackStore>(store: S, auth: AuthMiddleware) -> Router {
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

    let public_routes = Router::new()
        .route("/health", get(health))
        .route("/metrics", get(metrics::<S>))
        .route("/metrics/{identity}", get(metrics_single::<S>));

    Router::new()
        .merge(callback_routes)
        .merge(public_routes)
        .with_state(store)
        .layer(RequestBodyLimitLayer::new(1024)) // 1 KB — callbacks have no body
}
