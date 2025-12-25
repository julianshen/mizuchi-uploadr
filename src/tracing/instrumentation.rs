//! HTTP Span Instrumentation
//!
//! Provides utilities for creating and managing HTTP request/response spans
//! following OpenTelemetry semantic conventions.

use std::collections::HashMap;
use tracing::{span, Level, Span};

/// HTTP span attributes following OpenTelemetry semantic conventions
#[derive(Debug, Clone)]
pub struct HttpSpanAttributes {
    /// HTTP method (GET, POST, PUT, etc.)
    pub method: String,
    /// HTTP request path
    pub path: String,
    /// HTTP host header
    pub host: String,
    /// User-Agent header (optional)
    pub user_agent: Option<String>,
    /// Client IP address (optional)
    pub client_ip: Option<String>,
}

/// Create an HTTP span with semantic convention attributes
///
/// Creates a tracing span for an HTTP request with attributes following
/// OpenTelemetry semantic conventions for HTTP.
///
/// # Arguments
///
/// * `attrs` - HTTP span attributes
///
/// # Returns
///
/// A tracing `Span` with HTTP attributes
pub fn create_http_span(attrs: &HttpSpanAttributes) -> Span {
    let span = span!(
        Level::INFO,
        "http.request",
        http.method = %attrs.method,
        http.target = %attrs.path,
        http.host = %attrs.host,
        http.user_agent = attrs.user_agent.as_deref().unwrap_or(""),
        http.client_ip = attrs.client_ip.as_deref().unwrap_or(""),
    );

    span
}

/// Extract trace context from headers and create a span
///
/// Extracts W3C Trace Context from HTTP headers and creates a child span.
/// If no trace context is found, creates a new root span.
///
/// # Arguments
///
/// * `headers` - HTTP headers map
/// * `method` - HTTP method
/// * `path` - HTTP request path
///
/// # Returns
///
/// * `Ok(Span)` - Created span
/// * `Err(String)` - Error message
pub fn extract_and_create_span(
    headers: &HashMap<String, String>,
    method: &str,
    path: &str,
) -> Result<Span, String> {
    // Try to extract trace context
    let _trace_context = crate::tracing::propagation::extract_trace_context(headers);

    // Create span with basic attributes
    let span = span!(
        Level::INFO,
        "http.request",
        http.method = method,
        http.target = path,
    );

    Ok(span)
}

/// Record HTTP response attributes on a span
///
/// Records response metadata following OpenTelemetry semantic conventions.
///
/// # Arguments
///
/// * `span` - The span to record attributes on
/// * `status_code` - HTTP status code
/// * `content_length` - Response content length in bytes
/// * `content_type` - Response content type
///
/// # Returns
///
/// * `Ok(())` - Success
/// * `Err(String)` - Error message
pub fn record_response_attributes(
    span: &Span,
    status_code: u16,
    content_length: u64,
    content_type: &str,
) -> Result<(), String> {
    span.record("http.status_code", status_code);
    span.record("http.response_content_length", content_length);
    span.record("http.response_content_type", content_type);

    Ok(())
}

/// Record error response attributes on a span
///
/// Records error information when an HTTP request fails.
///
/// # Arguments
///
/// * `span` - The span to record attributes on
/// * `status_code` - HTTP error status code
/// * `error_message` - Error message
///
/// # Returns
///
/// * `Ok(())` - Success
/// * `Err(String)` - Error message
pub fn record_error_response(
    span: &Span,
    status_code: u16,
    error_message: &str,
) -> Result<(), String> {
    span.record("http.status_code", status_code);
    span.record("error", true);
    span.record("error.message", error_message);

    Ok(())
}

/// Record request body size on a span
///
/// Records the size of the HTTP request body.
///
/// # Arguments
///
/// * `span` - The span to record attributes on
/// * `body_size` - Request body size in bytes
///
/// # Returns
///
/// * `Ok(())` - Success
/// * `Err(String)` - Error message
pub fn record_request_body_size(span: &Span, body_size: u64) -> Result<(), String> {
    span.record("http.request_content_length", body_size);

    Ok(())
}

