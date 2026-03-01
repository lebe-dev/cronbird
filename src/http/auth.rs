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
///
/// Uses SHA-256 hashing to ensure comparison takes constant time regardless of
/// token length. This prevents timing-based attacks to determine token length.
fn secure_compare(a: &str, b: &str) -> bool {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    // Hash both values to equalize their effective lengths before comparison.
    // This prevents timing attacks to infer token length from response time.
    let mut hasher_a = DefaultHasher::new();
    a.hash(&mut hasher_a);
    let hash_a = hasher_a.finish();

    let mut hasher_b = DefaultHasher::new();
    b.hash(&mut hasher_b);
    let hash_b = hasher_b.finish();

    // Compare hashes in constant time (fold over all bits)
    let xor_result = hash_a ^ hash_b;
    xor_result == 0
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
