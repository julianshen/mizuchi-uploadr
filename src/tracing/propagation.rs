//! W3C Trace Context Propagation
//!
//! Implements W3C Trace Context specification for distributed tracing.
//! Extracts trace context from incoming HTTP requests and injects it into outgoing S3 requests.
//!
//! # W3C Trace Context Specification
//!
//! This module implements the [W3C Trace Context](https://www.w3.org/TR/trace-context/) specification
//! for propagating trace context across service boundaries.
//!
//! ## Headers
//!
//! - **traceparent**: Required header containing trace ID, span ID, and flags
//!   - Format: `00-{trace-id}-{span-id}-{trace-flags}`
//!   - Example: `00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01`
//!
//! - **tracestate**: Optional header for vendor-specific trace data
//!   - Format: `vendor1=value1,vendor2=value2`
//!   - Example: `congo=t61rcWkgMzE,rojo=00f067aa0ba902b7`
//!
//! ## Usage
//!
//! ### Extract from Incoming Request
//!
//! ```
//! use std::collections::HashMap;
//! use mizuchi_uploadr::tracing::propagation::extract_trace_context;
//!
//! let mut headers = HashMap::new();
//! headers.insert(
//!     "traceparent".to_string(),
//!     "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".to_string(),
//! );
//!
//! if let Some(context) = extract_trace_context(&headers) {
//!     println!("Trace ID: {}", context.trace_id);
//!     println!("Span ID: {}", context.span_id);
//!     println!("Sampled: {}", context.is_sampled());
//! }
//! ```
//!
//! ### Inject into Outgoing Request
//!
//! ```
//! use std::collections::HashMap;
//! use mizuchi_uploadr::tracing::propagation::{TraceContext, inject_trace_context};
//!
//! let context = TraceContext {
//!     trace_id: "0af7651916cd43dd8448eb211c80319c".to_string(),
//!     span_id: "b7ad6b7169203331".to_string(),
//!     trace_flags: 0x01,
//!     tracestate: None,
//! };
//!
//! let mut headers = HashMap::new();
//! inject_trace_context(&context, &mut headers);
//!
//! // Headers now contain traceparent
//! assert!(headers.contains_key("traceparent"));
//! ```

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

impl TraceContext {
    /// Check if the trace is sampled
    ///
    /// Returns true if the sampled flag (bit 0) is set
    pub fn is_sampled(&self) -> bool {
        (self.trace_flags & 0x01) != 0
    }

    /// Set the sampled flag
    ///
    /// Sets or clears the sampled flag (bit 0)
    pub fn set_sampled(&mut self, sampled: bool) {
        if sampled {
            self.trace_flags |= 0x01;
        } else {
            self.trace_flags &= !0x01;
        }
    }

    /// Format as traceparent header value
    ///
    /// Returns the traceparent header value in W3C format
    pub fn to_traceparent(&self) -> String {
        format!(
            "00-{}-{}-{:02x}",
            self.trace_id, self.span_id, self.trace_flags
        )
    }
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

    #[test]
    fn test_is_sampled() {
        let context = TraceContext {
            trace_id: "0af7651916cd43dd8448eb211c80319c".to_string(),
            span_id: "b7ad6b7169203331".to_string(),
            trace_flags: 0x01,
            tracestate: None,
        };
        assert!(context.is_sampled());

        let context_not_sampled = TraceContext {
            trace_id: "0af7651916cd43dd8448eb211c80319c".to_string(),
            span_id: "b7ad6b7169203331".to_string(),
            trace_flags: 0x00,
            tracestate: None,
        };
        assert!(!context_not_sampled.is_sampled());
    }

    #[test]
    fn test_set_sampled() {
        let mut context = TraceContext {
            trace_id: "0af7651916cd43dd8448eb211c80319c".to_string(),
            span_id: "b7ad6b7169203331".to_string(),
            trace_flags: 0x00,
            tracestate: None,
        };

        assert!(!context.is_sampled());

        context.set_sampled(true);
        assert!(context.is_sampled());
        assert_eq!(context.trace_flags, 0x01);

        context.set_sampled(false);
        assert!(!context.is_sampled());
        assert_eq!(context.trace_flags, 0x00);
    }

    #[test]
    fn test_to_traceparent() {
        let context = TraceContext {
            trace_id: "0af7651916cd43dd8448eb211c80319c".to_string(),
            span_id: "b7ad6b7169203331".to_string(),
            trace_flags: 0x01,
            tracestate: None,
        };

        assert_eq!(
            context.to_traceparent(),
            "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01"
        );
    }
}
