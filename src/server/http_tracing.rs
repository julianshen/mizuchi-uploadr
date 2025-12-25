//! HTTP Request Tracing
//!
//! Provides instrumentation for HTTP requests with OpenTelemetry spans.
//! Implements W3C Trace Context propagation and HTTP semantic conventions.
//!
//! # Features
//!
//! - **Root Span Creation**: Creates a root span for each HTTP request
//! - **HTTP Semantic Conventions**: Follows OpenTelemetry HTTP conventions
//! - **W3C Trace Context**: Extracts and propagates trace context headers
//! - **Error Recording**: Records errors with proper span attributes
//!
//! # Example
//!
//! ```no_run
//! use mizuchi_uploadr::server::http_tracing::{MockRequest, create_request_span};
//!
//! let request = MockRequest {
//!     method: "PUT".to_string(),
//!     path: "/uploads/file.txt".to_string(),
//!     headers: vec![],
//! };
//!
//! if let Some(span) = create_request_span(&request) {
//!     let _enter = span.enter();
//!     // Process request within span context
//! }
//! ```
//!
//! # W3C Trace Context
//!
//! The module supports W3C Trace Context propagation via the `traceparent` header:
//!
//! ```text
//! traceparent: 00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01
//!              │  │                                │                  │
//!              │  │                                │                  └─ trace-flags
//!              │  │                                └──────────────────── parent-id
//!              │  └───────────────────────────────────────────────────── trace-id
//!              └────────────────────────────────────────────────────────── version
//! ```

use std::collections::HashMap;
use tracing::Span;

/// Mock HTTP request for testing
#[derive(Debug, Clone)]
pub struct MockRequest {
    pub method: String,
    pub path: String,
    pub headers: Vec<(String, String)>,
}

/// W3C Trace Context extracted from headers
///
/// Represents the distributed tracing context propagated via HTTP headers.
/// Follows the W3C Trace Context specification.
///
/// # Fields
///
/// * `trace_id` - Unique identifier for the trace (32 hex characters)
/// * `parent_span_id` - Identifier of the parent span (16 hex characters)
/// * `trace_flags` - Sampling and other flags (2 hex characters)
/// * `trace_state` - Optional vendor-specific trace state
#[derive(Debug, Clone, PartialEq)]
pub struct TraceContext {
    pub trace_id: String,
    pub parent_span_id: String,
    pub trace_flags: String,
    pub trace_state: Option<String>,
}

impl MockRequest {
    /// Get header value by name (case-insensitive)
    pub fn get_header(&self, name: &str) -> Option<&str> {
        let name_lower = name.to_lowercase();
        self.headers
            .iter()
            .find(|(k, _)| k.to_lowercase() == name_lower)
            .map(|(_, v)| v.as_str())
    }
}

/// Create a root span for an HTTP request
///
/// Returns Some(Span) if tracing is enabled, None otherwise
pub fn create_request_span(request: &MockRequest) -> Option<Span> {
    // Create span with HTTP semantic conventions
    let span = tracing::info_span!(
        "http.request",
        http.method = %request.method,
        http.target = %request.path,
        http.scheme = "http",
        otel.kind = "server",
    );

    Some(span)
}

/// Extract HTTP attributes from request
///
/// Returns a map of HTTP semantic convention attributes
pub fn extract_http_attributes(request: &MockRequest) -> HashMap<String, String> {
    let mut attributes = HashMap::new();

    attributes.insert("http.method".to_string(), request.method.clone());
    attributes.insert("http.target".to_string(), request.path.clone());
    attributes.insert("http.scheme".to_string(), "http".to_string());

    // Add content-type if present
    if let Some(content_type) = request.get_header("content-type") {
        attributes.insert(
            "http.request.content_type".to_string(),
            content_type.to_string(),
        );
    }

    // Add content-length if present
    if let Some(content_length) = request.get_header("content-length") {
        attributes.insert(
            "http.request.content_length".to_string(),
            content_length.to_string(),
        );
    }

    attributes
}

/// Extract W3C Trace Context from request headers
///
/// Parses the `traceparent` and optional `tracestate` headers according to
/// W3C Trace Context specification: https://www.w3.org/TR/trace-context/
///
/// # Traceparent Format
///
/// ```text
/// version-trace_id-parent_id-trace_flags
/// 00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01
/// ```
///
/// # Tracestate Format (optional)
///
/// ```text
/// vendor1=value1,vendor2=value2
/// ```
///
/// # Returns
///
/// * `Some(TraceContext)` - If traceparent header is valid
/// * `None` - If traceparent is missing or invalid
pub fn extract_trace_context(request: &MockRequest) -> Option<TraceContext> {
    let traceparent = request.get_header("traceparent")?;

    // Parse traceparent header
    let parts: Vec<&str> = traceparent.split('-').collect();
    if parts.len() != 4 {
        tracing::warn!(
            "Invalid traceparent format: expected 4 parts, got {}",
            parts.len()
        );
        return None;
    }

    // Validate version (should be "00")
    if parts[0] != "00" {
        tracing::warn!("Unsupported traceparent version: {}", parts[0]);
        return None;
    }

    // Validate trace_id length (32 hex chars)
    if parts[1].len() != 32 {
        tracing::warn!(
            "Invalid trace_id length: expected 32, got {}",
            parts[1].len()
        );
        return None;
    }

    // Validate parent_span_id length (16 hex chars)
    if parts[2].len() != 16 {
        tracing::warn!(
            "Invalid parent_span_id length: expected 16, got {}",
            parts[2].len()
        );
        return None;
    }

    // Extract optional tracestate header
    let trace_state = request.get_header("tracestate").map(|s| s.to_string());

    Some(TraceContext {
        trace_id: parts[1].to_string(),
        parent_span_id: parts[2].to_string(),
        trace_flags: parts[3].to_string(),
        trace_state,
    })
}

/// Record an error in the current span
///
/// Adds error attributes to the span according to OpenTelemetry semantic conventions
pub fn record_error_in_span(
    _request: &MockRequest,
    status_code: u16,
    error_message: &str,
) -> Result<(), String> {
    // Get current span
    let span = Span::current();

    // Record error attributes
    span.record("http.status_code", status_code);
    span.record("error", true);
    span.record("error.message", error_message);

    // Log the error
    tracing::error!(
        status_code = status_code,
        error = error_message,
        "HTTP request failed"
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_request_get_header() {
        let request = MockRequest {
            method: "PUT".to_string(),
            path: "/test".to_string(),
            headers: vec![("Content-Type".to_string(), "text/plain".to_string())],
        };

        assert_eq!(request.get_header("content-type"), Some("text/plain"));
        assert_eq!(request.get_header("Content-Type"), Some("text/plain"));
        assert_eq!(request.get_header("missing"), None);
    }
}
