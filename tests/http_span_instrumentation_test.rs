//! HTTP Span Instrumentation Tests
//!
//! Tests for HTTP request/response span creation and instrumentation.
//! Validates that spans are created with correct attributes following
//! OpenTelemetry semantic conventions.

#[cfg(feature = "tracing")]
mod tests {
    use mizuchi_uploadr::tracing::instrumentation::{create_http_span, HttpSpanAttributes};
    use std::collections::HashMap;

    /// Test that HTTP span is created with basic attributes
    #[test]
    fn test_create_http_span_basic() {
        let attrs = HttpSpanAttributes {
            method: "PUT".to_string(),
            path: "/bucket/key".to_string(),
            host: "localhost:8080".to_string(),
            user_agent: Some("aws-sdk-rust/1.0".to_string()),
            client_ip: Some("192.168.1.100".to_string()),
        };

        let span = create_http_span(&attrs);
        
        // Span should be created
        assert!(!span.is_none());
    }

    /// Test that HTTP span includes semantic convention attributes
    #[test]
    fn test_http_span_semantic_conventions() {
        let attrs = HttpSpanAttributes {
            method: "POST".to_string(),
            path: "/uploads/file.txt".to_string(),
            host: "api.example.com".to_string(),
            user_agent: Some("curl/7.68.0".to_string()),
            client_ip: Some("10.0.0.1".to_string()),
        };

        let span = create_http_span(&attrs);
        
        // Verify span has correct attributes
        // This will fail until we implement the function
        assert!(!span.is_none());
    }

    /// Test that trace context is extracted from headers
    #[test]
    fn test_extract_trace_context_from_request() {
        let mut headers = HashMap::new();
        headers.insert(
            "traceparent".to_string(),
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".to_string(),
        );

        // This should extract the trace context and create a child span
        let result = mizuchi_uploadr::tracing::instrumentation::extract_and_create_span(
            &headers,
            "PUT",
            "/bucket/object.txt",
        );

        assert!(result.is_ok());
    }

    /// Test that span is created even without trace context
    #[test]
    fn test_create_span_without_trace_context() {
        let headers = HashMap::new();

        // Should create a new root span
        let result = mizuchi_uploadr::tracing::instrumentation::extract_and_create_span(
            &headers,
            "GET",
            "/health",
        );

        assert!(result.is_ok());
    }

    /// Test that HTTP response attributes are recorded
    #[test]
    fn test_record_http_response_attributes() {
        let attrs = HttpSpanAttributes {
            method: "PUT".to_string(),
            path: "/bucket/file.bin".to_string(),
            host: "s3.amazonaws.com".to_string(),
            user_agent: None,
            client_ip: None,
        };

        let span = create_http_span(&attrs);

        // Record response attributes
        let result = mizuchi_uploadr::tracing::instrumentation::record_response_attributes(
            &span,
            200,
            1024,
            "application/octet-stream",
        );

        assert!(result.is_ok());
    }

    /// Test that error responses are properly recorded
    #[test]
    fn test_record_error_response() {
        let attrs = HttpSpanAttributes {
            method: "PUT".to_string(),
            path: "/bucket/forbidden.txt".to_string(),
            host: "localhost:8080".to_string(),
            user_agent: None,
            client_ip: None,
        };

        let span = create_http_span(&attrs);

        // Record error response
        let result = mizuchi_uploadr::tracing::instrumentation::record_error_response(
            &span,
            403,
            "Access Denied",
        );

        assert!(result.is_ok());
    }

    /// Test that request body size is tracked
    #[test]
    fn test_track_request_body_size() {
        let attrs = HttpSpanAttributes {
            method: "PUT".to_string(),
            path: "/bucket/large-file.bin".to_string(),
            host: "localhost:8080".to_string(),
            user_agent: None,
            client_ip: None,
        };

        let span = create_http_span(&attrs);

        // Track body size
        let result = mizuchi_uploadr::tracing::instrumentation::record_request_body_size(
            &span,
            104857600, // 100MB
        );

        assert!(result.is_ok());
    }
}

