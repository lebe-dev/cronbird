use axum::{
    extract::Request,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};

/// Authentication middleware for Bearer token validation.
/// If no token is configured, this middleware is a no-op passthrough.
#[derive(Clone)]
pub struct AuthMiddleware {
    token: Option<String>,
}

impl AuthMiddleware {
    /// Creates a new AuthMiddleware.
    /// If token is None, authentication is disabled.
    pub fn new(token: Option<String>) -> Self {
        Self { token }
    }

    /// Middleware handler function.
    pub async fn handle(
        &self,
        headers: HeaderMap,
        request: Request,
        next: Next,
    ) -> Result<Response, StatusCode> {
        // If no token is configured, allow all requests
        let Some(expected_token) = &self.token else {
            return Ok(next.run(request).await);
        };

        // Extract Authorization header
        let auth_header = headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .ok_or(StatusCode::UNAUTHORIZED)?;

        // Check Bearer token format
        if !auth_header.starts_with("Bearer ") {
            return Err(StatusCode::UNAUTHORIZED);
        }

        let provided_token = &auth_header[7..]; // Skip "Bearer "

        // Constant-time comparison to prevent timing attacks
        if !secure_compare(provided_token, expected_token) {
            return Err(StatusCode::UNAUTHORIZED);
        }

        Ok(next.run(request).await)
    }

    /// Returns true if authentication is enabled.
    pub fn is_enabled(&self) -> bool {
        self.token.is_some()
    }
}

/// Constant-time string comparison to prevent timing attacks.
fn secure_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    a.bytes()
        .zip(b.bytes())
        .fold(0, |acc, (a, b)| acc | (a ^ b))
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_compare() {
        assert!(secure_compare("secret", "secret"));
        assert!(!secure_compare("secret", "public"));
        assert!(!secure_compare("secret", "Secret"));
        assert!(!secure_compare("short", "longer"));
    }

    #[test]
    fn test_auth_middleware_disabled() {
        let middleware = AuthMiddleware::new(None);
        assert!(!middleware.is_enabled());
    }

    #[test]
    fn test_auth_middleware_enabled() {
        let middleware = AuthMiddleware::new(Some("secret-token".to_string()));
        assert!(middleware.is_enabled());
    }
}
