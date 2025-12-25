//! HTTP Span Instrumentation
//!
//! Provides utilities for creating and managing HTTP request/response spans
//! following OpenTelemetry semantic conventions.
//!
//! # Overview
//!
//! This module implements HTTP instrumentation following the
//! [OpenTelemetry Semantic Conventions for HTTP](https://opentelemetry.io/docs/specs/semconv/http/).
//!
//! ## Features
//!
//! - **Automatic span creation** for HTTP requests
//! - **W3C Trace Context propagation** from incoming requests
//! - **Semantic convention attributes** for HTTP metadata
//! - **Response tracking** with status codes and content length
//! - **Error recording** with structured error information
//!
//! # Example
//!
//! ```
//! use mizuchi_uploadr::tracing::instrumentation::{HttpSpanAttributes, create_http_span};
//!
//! let attrs = HttpSpanAttributes {
//!     method: "PUT".to_string(),
//!     path: "/bucket/object.txt".to_string(),
//!     host: "localhost:8080".to_string(),
//!     user_agent: Some("aws-sdk-rust/1.0".to_string()),
//!     client_ip: Some("192.168.1.100".to_string()),
//! };
//!
//! let span = create_http_span(&attrs);
//! // Use the span for the request lifecycle
//! ```
//!
//! # Semantic Conventions
//!
//! This module follows OpenTelemetry semantic conventions:
//!
//! | Attribute | Description | Example |
//! |-----------|-------------|---------|
//! | `http.method` | HTTP method | `PUT` |
//! | `http.target` | Request path | `/bucket/key` |
//! | `http.host` | Host header | `localhost:8080` |
//! | `http.user_agent` | User-Agent header | `aws-sdk-rust/1.0` |
//! | `http.client_ip` | Client IP address | `192.168.1.100` |
//! | `http.status_code` | Response status | `200` |
//! | `http.response_content_length` | Response size | `1024` |
//! | `http.request_content_length` | Request size | `104857600` |

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
    // Create span with basic attributes
    let span = span!(
        Level::INFO,
        "http.request",
        http.method = method,
        http.target = path,
    );

    // Try to extract trace context and link parent span
    if let Some(trace_context) = crate::tracing::propagation::extract_trace_context(headers) {
        // Convert our TraceContext to OpenTelemetry Context using propagator
        use opentelemetry::propagation::{Extractor, TextMapPropagator};
        use opentelemetry_sdk::propagation::TraceContextPropagator;
        use tracing_opentelemetry::OpenTelemetrySpanExt;

        // Create a simple extractor from our trace context
        struct SimpleExtractor {
            traceparent: String,
            tracestate: Option<String>,
        }

        impl Extractor for SimpleExtractor {
            fn get(&self, key: &str) -> Option<&str> {
                match key.to_lowercase().as_str() {
                    "traceparent" => Some(&self.traceparent),
                    "tracestate" => self.tracestate.as_deref(),
                    _ => None,
                }
            }

            fn keys(&self) -> Vec<&str> {
                let mut keys = vec!["traceparent"];
                if self.tracestate.is_some() {
                    keys.push("tracestate");
                }
                keys
            }
        }

        let extractor = SimpleExtractor {
            traceparent: trace_context.to_traceparent(),
            tracestate: trace_context.tracestate.clone(),
        };

        // Extract OpenTelemetry context using W3C TraceContext propagator
        let propagator = TraceContextPropagator::new();
        let parent_context = propagator.extract(&extractor);

        // Set the parent context on the span
        span.set_parent(parent_context);
    }

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
///
/// # Example
///
/// ```
/// use mizuchi_uploadr::tracing::instrumentation::{HttpSpanAttributes, create_http_span, record_request_body_size};
///
/// let attrs = HttpSpanAttributes {
///     method: "PUT".to_string(),
///     path: "/bucket/large-file.bin".to_string(),
///     host: "localhost:8080".to_string(),
///     user_agent: None,
///     client_ip: None,
/// };
///
/// let span = create_http_span(&attrs);
/// record_request_body_size(&span, 104857600).unwrap(); // 100MB
/// ```
pub fn record_request_body_size(span: &Span, body_size: u64) -> Result<(), String> {
    span.record("http.request_content_length", body_size);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_span_attributes_creation() {
        let attrs = HttpSpanAttributes {
            method: "GET".to_string(),
            path: "/health".to_string(),
            host: "localhost:8080".to_string(),
            user_agent: Some("test-client/1.0".to_string()),
            client_ip: Some("127.0.0.1".to_string()),
        };

        assert_eq!(attrs.method, "GET");
        assert_eq!(attrs.path, "/health");
        assert_eq!(attrs.host, "localhost:8080");
        assert_eq!(attrs.user_agent, Some("test-client/1.0".to_string()));
        assert_eq!(attrs.client_ip, Some("127.0.0.1".to_string()));
    }

    #[test]
    fn test_create_span_with_minimal_attributes() {
        let attrs = HttpSpanAttributes {
            method: "POST".to_string(),
            path: "/api/upload".to_string(),
            host: "api.example.com".to_string(),
            user_agent: None,
            client_ip: None,
        };

        let _span = create_http_span(&attrs);
        // Span creation succeeds
    }

    #[test]
    fn test_extract_span_with_empty_headers() {
        let headers = HashMap::new();
        let result = extract_and_create_span(&headers, "GET", "/");
        assert!(result.is_ok());
    }

    #[test]
    fn test_record_response_success() {
        let attrs = HttpSpanAttributes {
            method: "PUT".to_string(),
            path: "/bucket/file.txt".to_string(),
            host: "localhost:8080".to_string(),
            user_agent: None,
            client_ip: None,
        };

        let span = create_http_span(&attrs);
        let result = record_response_attributes(&span, 200, 1024, "text/plain");
        assert!(result.is_ok());
    }

    #[test]
    fn test_record_error_success() {
        let attrs = HttpSpanAttributes {
            method: "GET".to_string(),
            path: "/forbidden".to_string(),
            host: "localhost:8080".to_string(),
            user_agent: None,
            client_ip: None,
        };

        let span = create_http_span(&attrs);
        let result = record_error_response(&span, 403, "Forbidden");
        assert!(result.is_ok());
    }

    #[test]
    fn test_record_body_size_success() {
        let attrs = HttpSpanAttributes {
            method: "PUT".to_string(),
            path: "/upload".to_string(),
            host: "localhost:8080".to_string(),
            user_agent: None,
            client_ip: None,
        };

        let span = create_http_span(&attrs);
        let result = record_request_body_size(&span, 1048576);
        assert!(result.is_ok());
    }
}
