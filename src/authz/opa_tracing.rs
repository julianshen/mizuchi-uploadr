//! OPA Authorization Tracing
//!
//! Provides instrumentation for Open Policy Agent (OPA) authorization.
//! Ensures no PII (user identities, sensitive resources) is leaked in spans.

use std::collections::HashMap;
use tracing::Span;

/// Mock authorization decision for testing
#[derive(Debug, Clone)]
pub struct MockAuthzDecision {
    pub subject: String,
    pub action: String,
    pub resource: String,
    pub allowed: bool,
}

/// Create a span for OPA authorization
///
/// Returns Some(Span) if tracing is enabled, None otherwise
pub fn create_opa_authz_span(decision: &MockAuthzDecision) -> Option<Span> {
    // Create span for OPA authorization
    let decision_str = if decision.allowed { "allow" } else { "deny" };
    
    let span = tracing::info_span!(
        "authz.opa",
        authz.provider = "opa",
        authz.decision = decision_str,
        authz.action = %decision.action,
        otel.kind = "internal",
    );
    
    Some(span)
}

/// Extract OPA authorization attributes (no PII)
///
/// Returns a map of authorization attributes safe for tracing.
/// Ensures no user identities or sensitive resource paths are included.
pub fn extract_authz_attributes(decision: &MockAuthzDecision) -> HashMap<String, String> {
    let mut attributes = HashMap::new();
    
    // Record decision (allow/deny)
    let decision_str = if decision.allowed { "allow" } else { "deny" };
    attributes.insert("authz.decision".to_string(), decision_str.to_string());
    
    // Record action (safe to include)
    attributes.insert("authz.action".to_string(), decision.action.clone());
    
    // Record resource type (sanitized)
    let resource_type = sanitize_resource_path(&decision.resource);
    attributes.insert("authz.resource_type".to_string(), resource_type);
    
    // Do NOT include:
    // - Subject (user email, username, user ID)
    // - Full resource path (may contain sensitive filenames)
    // - Any policy details
    
    attributes
}

/// Sanitize resource path to remove sensitive information
///
/// Extracts only the resource type (e.g., "/uploads/file.txt" -> "uploads")
fn sanitize_resource_path(path: &str) -> String {
    let path = path.trim_start_matches('/');
    let parts: Vec<&str> = path.split('/').collect();
    
    if parts.is_empty() {
        return "unknown".to_string();
    }
    
    // Return only the first path segment (bucket/prefix)
    parts[0].to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_resource_path() {
        assert_eq!(sanitize_resource_path("/uploads/file.txt"), "uploads");
        assert_eq!(sanitize_resource_path("/uploads/sensitive/secret.txt"), "uploads");
        assert_eq!(sanitize_resource_path("uploads/file.txt"), "uploads");
        assert_eq!(sanitize_resource_path(""), "unknown");
    }

    #[test]
    fn test_extract_authz_attributes_no_pii() {
        let decision = MockAuthzDecision {
            subject: "user@example.com".to_string(), // PII
            action: "upload".to_string(),
            resource: "/uploads/sensitive.txt".to_string(),
            allowed: false,
        };
        
        let attrs = extract_authz_attributes(&decision);
        
        // Should have decision and action
        assert_eq!(attrs.get("authz.decision"), Some(&"deny".to_string()));
        assert_eq!(attrs.get("authz.action"), Some(&"upload".to_string()));
        assert_eq!(attrs.get("authz.resource_type"), Some(&"uploads".to_string()));
        
        // Should NOT have user email
        assert!(!attrs.contains_key("authz.subject"));
        assert!(!attrs.values().any(|v| v.contains("user@example.com")));
        
        // Should NOT have sensitive filename
        assert!(!attrs.values().any(|v| v.contains("sensitive.txt")));
    }

    #[test]
    fn test_create_opa_authz_span_allow() {
        let decision = MockAuthzDecision {
            subject: "user123".to_string(),
            action: "read".to_string(),
            resource: "/data/file.txt".to_string(),
            allowed: true,
        };
        
        let span = create_opa_authz_span(&decision);
        assert!(span.is_some());
    }

    #[test]
    fn test_create_opa_authz_span_deny() {
        let decision = MockAuthzDecision {
            subject: "user123".to_string(),
            action: "delete".to_string(),
            resource: "/data/file.txt".to_string(),
            allowed: false,
        };
        
        let span = create_opa_authz_span(&decision);
        assert!(span.is_some());
    }
}

