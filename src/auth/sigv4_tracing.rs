//! AWS SigV4 Authentication Tracing
//!
//! Provides instrumentation for AWS Signature Version 4 authentication.
//! Ensures no credentials or sensitive information is leaked in spans.

use std::collections::HashMap;
use tracing::Span;

// Re-export MockAuthRequest from jwt_tracing
pub use super::jwt_tracing::MockAuthRequest;

/// Create a span for SigV4 authentication
///
/// Returns Some(Span) if tracing is enabled, None otherwise
pub fn create_sigv4_auth_span(request: &MockAuthRequest) -> Option<Span> {
    // Create span for SigV4 authentication
    let span = tracing::info_span!(
        "auth.sigv4",
        auth.method = "sigv4",
        auth.signature_present = %has_sigv4_signature(request),
        otel.kind = "internal",
    );

    Some(span)
}

/// Check if request has a SigV4 signature
fn has_sigv4_signature(request: &MockAuthRequest) -> bool {
    if let Some(auth) = request.get_header("authorization") {
        return auth.starts_with("AWS4-HMAC-SHA256");
    }
    false
}

/// Extract SigV4 authentication attributes (no PII)
///
/// Returns a map of authentication attributes safe for tracing.
/// Ensures no access keys, secret keys, or signatures are included.
pub fn extract_sigv4_attributes(request: &MockAuthRequest) -> HashMap<String, String> {
    let mut attributes = HashMap::new();

    attributes.insert("auth.method".to_string(), "sigv4".to_string());

    // Only record presence of signature, not the actual signature
    let signature_present = has_sigv4_signature(request);
    attributes.insert(
        "auth.signature_present".to_string(),
        signature_present.to_string(),
    );

    // Do NOT include:
    // - Access key ID
    // - Secret access key
    // - Signature value
    // - Credential scope

    attributes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_sigv4_signature() {
        let request = MockAuthRequest {
            headers: vec![(
                "authorization".to_string(),
                "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/...".to_string(),
            )],
            method: "PUT".to_string(),
            path: "/test".to_string(),
        };

        assert!(has_sigv4_signature(&request));
    }

    #[test]
    fn test_no_sigv4_signature() {
        let request = MockAuthRequest {
            headers: vec![],
            method: "PUT".to_string(),
            path: "/test".to_string(),
        };

        assert!(!has_sigv4_signature(&request));
    }

    #[test]
    fn test_extract_sigv4_attributes_no_credentials() {
        let request = MockAuthRequest {
            headers: vec![
                ("authorization".to_string(), 
                 "AWS4-HMAC-SHA256 Credential=AKIAIOSFODNN7EXAMPLE/20230101/us-east-1/s3/aws4_request".to_string()),
            ],
            method: "PUT".to_string(),
            path: "/test".to_string(),
        };

        let attrs = extract_sigv4_attributes(&request);

        // Should have method and signature presence
        assert_eq!(attrs.get("auth.method"), Some(&"sigv4".to_string()));
        assert_eq!(
            attrs.get("auth.signature_present"),
            Some(&"true".to_string())
        );

        // Should NOT have access key or credentials
        assert!(!attrs.contains_key("auth.access_key"));
        assert!(!attrs.values().any(|v| v.contains("AKIAIOSFODNN7EXAMPLE")));
    }
}
