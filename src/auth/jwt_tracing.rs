//! JWT Authentication Tracing
//!
//! Provides instrumentation for JWT authentication with OpenTelemetry spans.
//! Ensures no PII (Personally Identifiable Information) is leaked in spans.

use std::collections::HashMap;
use tracing::Span;

/// Mock authentication request for testing
#[derive(Debug, Clone)]
pub struct MockAuthRequest {
    pub headers: Vec<(String, String)>,
    pub method: String,
    pub path: String,
}

impl MockAuthRequest {
    /// Get header value by name (case-insensitive)
    pub fn get_header(&self, name: &str) -> Option<&str> {
        let name_lower = name.to_lowercase();
        self.headers
            .iter()
            .find(|(k, _)| k.to_lowercase() == name_lower)
            .map(|(_, v)| v.as_str())
    }
}

/// Create a span for JWT authentication
///
/// Returns Some(Span) if tracing is enabled, None otherwise
pub fn create_jwt_auth_span(request: &MockAuthRequest) -> Option<Span> {
    // Create span for JWT authentication
    let span = tracing::info_span!(
        "auth.jwt",
        auth.method = "jwt",
        auth.token_present = %has_bearer_token(request),
        otel.kind = "internal",
    );
    
    Some(span)
}

/// Check if request has a Bearer token
fn has_bearer_token(request: &MockAuthRequest) -> bool {
    if let Some(auth) = request.get_header("authorization") {
        return auth.starts_with("Bearer ");
    }
    false
}

/// Extract JWT authentication attributes (no PII)
///
/// Returns a map of authentication attributes safe for tracing.
/// Ensures no tokens, emails, or other PII are included.
pub fn extract_jwt_attributes(request: &MockAuthRequest) -> HashMap<String, String> {
    let mut attributes = HashMap::new();
    
    attributes.insert("auth.method".to_string(), "jwt".to_string());
    
    // Only record presence of token, not the actual token
    let token_present = has_bearer_token(request);
    attributes.insert("auth.token_present".to_string(), token_present.to_string());
    
    // Do NOT include:
    // - Actual token value
    // - User email or username
    // - Any claims from the JWT
    
    attributes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_bearer_token() {
        let request = MockAuthRequest {
            headers: vec![
                ("authorization".to_string(), "Bearer token123".to_string()),
            ],
            method: "PUT".to_string(),
            path: "/test".to_string(),
        };
        
        assert!(has_bearer_token(&request));
    }

    #[test]
    fn test_no_bearer_token() {
        let request = MockAuthRequest {
            headers: vec![],
            method: "PUT".to_string(),
            path: "/test".to_string(),
        };
        
        assert!(!has_bearer_token(&request));
    }

    #[test]
    fn test_extract_jwt_attributes_no_pii() {
        let request = MockAuthRequest {
            headers: vec![
                ("authorization".to_string(), "Bearer secret_token_123".to_string()),
            ],
            method: "PUT".to_string(),
            path: "/test".to_string(),
        };
        
        let attrs = extract_jwt_attributes(&request);
        
        // Should have method and token presence
        assert_eq!(attrs.get("auth.method"), Some(&"jwt".to_string()));
        assert_eq!(attrs.get("auth.token_present"), Some(&"true".to_string()));
        
        // Should NOT have actual token
        assert!(!attrs.contains_key("auth.token"));
        assert!(!attrs.values().any(|v| v.contains("secret_token")));
    }
}

