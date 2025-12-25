//! W3C Trace Context Propagation
//!
//! Implements W3C Trace Context specification for distributed tracing.
//! Extracts trace context from incoming HTTP requests and injects it into outgoing S3 requests.

use std::collections::HashMap;

/// W3C Trace Context
///
/// Represents a W3C Trace Context with trace ID, span ID, trace flags, and optional tracestate.
///
/// Format: `traceparent: 00-{trace-id}-{span-id}-{trace-flags}`
#[derive(Debug, Clone, PartialEq)]
pub struct TraceContext {
    /// Trace ID (32 hex characters)
    pub trace_id: String,
    /// Span ID (16 hex characters)
    pub span_id: String,
    /// Trace flags (8-bit field)
    pub trace_flags: u8,
    /// Optional tracestate header value
    pub tracestate: Option<String>,
}

/// Extract trace context from HTTP headers
///
/// Parses the `traceparent` and optional `tracestate` headers according to
/// W3C Trace Context specification.
///
/// # Arguments
///
/// * `headers` - HTTP headers map
///
/// # Returns
///
/// * `Some(TraceContext)` if valid traceparent header is found
/// * `None` if traceparent is missing or invalid
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
/// use mizuchi_uploadr::tracing::propagation::extract_trace_context;
///
/// let mut headers = HashMap::new();
/// headers.insert(
///     "traceparent".to_string(),
///     "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".to_string(),
/// );
///
/// let context = extract_trace_context(&headers);
/// assert!(context.is_some());
/// ```
pub fn extract_trace_context(headers: &HashMap<String, String>) -> Option<TraceContext> {
    // Get traceparent header (case-insensitive)
    let traceparent = headers
        .iter()
        .find(|(k, _)| k.to_lowercase() == "traceparent")
        .map(|(_, v)| v)?;

    // Parse traceparent: version-trace_id-span_id-trace_flags
    let parts: Vec<&str> = traceparent.split('-').collect();
    if parts.len() != 4 {
        return None;
    }

    // Validate version (must be "00")
    if parts[0] != "00" {
        return None;
    }

    // Validate trace_id (32 hex chars)
    if parts[1].len() != 32 || !parts[1].chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }

    // Validate span_id (16 hex chars)
    if parts[2].len() != 16 || !parts[2].chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }

    // Parse trace_flags (2 hex chars = 1 byte)
    let trace_flags = u8::from_str_radix(parts[3], 16).ok()?;

    // Get optional tracestate header
    let tracestate = headers
        .iter()
        .find(|(k, _)| k.to_lowercase() == "tracestate")
        .map(|(_, v)| v.clone());

    Some(TraceContext {
        trace_id: parts[1].to_string(),
        span_id: parts[2].to_string(),
        trace_flags,
        tracestate,
    })
}

/// Inject trace context into HTTP headers
///
/// Formats the trace context as `traceparent` and optional `tracestate` headers
/// according to W3C Trace Context specification.
///
/// # Arguments
///
/// * `context` - Trace context to inject
/// * `headers` - HTTP headers map to inject into
///
/// # Example
///
/// ```
/// use std::collections::HashMap;
/// use mizuchi_uploadr::tracing::propagation::{TraceContext, inject_trace_context};
///
/// let context = TraceContext {
///     trace_id: "0af7651916cd43dd8448eb211c80319c".to_string(),
///     span_id: "b7ad6b7169203331".to_string(),
///     trace_flags: 0x01,
///     tracestate: None,
/// };
///
/// let mut headers = HashMap::new();
/// inject_trace_context(&context, &mut headers);
///
/// assert!(headers.contains_key("traceparent"));
/// ```
pub fn inject_trace_context(context: &TraceContext, headers: &mut HashMap<String, String>) {
    // Format traceparent: version-trace_id-span_id-trace_flags
    let traceparent = format!(
        "00-{}-{}-{:02x}",
        context.trace_id, context.span_id, context.trace_flags
    );

    headers.insert("traceparent".to_string(), traceparent);

    // Add tracestate if present
    if let Some(ref tracestate) = context.tracestate {
        headers.insert("tracestate".to_string(), tracestate.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_traceparent() {
        let mut headers = HashMap::new();
        headers.insert(
            "traceparent".to_string(),
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".to_string(),
        );

        let context = extract_trace_context(&headers).unwrap();
        assert_eq!(context.trace_id, "0af7651916cd43dd8448eb211c80319c");
        assert_eq!(context.span_id, "b7ad6b7169203331");
        assert_eq!(context.trace_flags, 0x01);
    }
}

