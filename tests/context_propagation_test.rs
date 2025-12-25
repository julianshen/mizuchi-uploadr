//! W3C Trace Context Propagation Tests
//!
//! Tests for extracting trace context from incoming HTTP requests
//! and injecting it into outgoing S3 requests.
//!
//! TDD Phase: RED - These tests should FAIL initially

#[cfg(feature = "tracing")]
use mizuchi_uploadr::tracing::propagation::{extract_trace_context, inject_trace_context};
use std::collections::HashMap;

#[cfg(feature = "tracing")]
#[test]
fn test_extract_valid_traceparent() {
    // RED: extract_trace_context doesn't exist yet
    let mut headers = HashMap::new();
    headers.insert(
        "traceparent".to_string(),
        "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".to_string(),
    );

    let context = extract_trace_context(&headers);
    assert!(context.is_some());

    let ctx = context.unwrap();
    assert_eq!(ctx.trace_id, "0af7651916cd43dd8448eb211c80319c");
    assert_eq!(ctx.span_id, "b7ad6b7169203331");
    assert_eq!(ctx.trace_flags, 0x01); // sampled
}

#[cfg(feature = "tracing")]
#[test]
fn test_extract_traceparent_with_tracestate() {
    // RED: extract_trace_context doesn't exist yet
    let mut headers = HashMap::new();
    headers.insert(
        "traceparent".to_string(),
        "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".to_string(),
    );
    headers.insert(
        "tracestate".to_string(),
        "congo=t61rcWkgMzE,rojo=00f067aa0ba902b7".to_string(),
    );

    let context = extract_trace_context(&headers);
    assert!(context.is_some());

    let ctx = context.unwrap();
    assert_eq!(ctx.trace_id, "0af7651916cd43dd8448eb211c80319c");
    assert!(ctx.tracestate.is_some());
    assert_eq!(
        ctx.tracestate.unwrap(),
        "congo=t61rcWkgMzE,rojo=00f067aa0ba902b7"
    );
}

#[cfg(feature = "tracing")]
#[test]
fn test_extract_missing_traceparent() {
    // RED: extract_trace_context doesn't exist yet
    let headers = HashMap::new();

    let context = extract_trace_context(&headers);
    assert!(context.is_none());
}

#[cfg(feature = "tracing")]
#[test]
fn test_extract_invalid_traceparent() {
    // RED: extract_trace_context doesn't exist yet
    let mut headers = HashMap::new();
    headers.insert("traceparent".to_string(), "invalid".to_string());

    let context = extract_trace_context(&headers);
    assert!(context.is_none());
}

#[cfg(feature = "tracing")]
#[test]
fn test_inject_trace_context() {
    // RED: inject_trace_context doesn't exist yet
    use mizuchi_uploadr::tracing::propagation::TraceContext;

    let context = TraceContext {
        trace_id: "0af7651916cd43dd8448eb211c80319c".to_string(),
        span_id: "b7ad6b7169203331".to_string(),
        trace_flags: 0x01,
        tracestate: Some("congo=t61rcWkgMzE".to_string()),
    };

    let mut headers = HashMap::new();
    inject_trace_context(&context, &mut headers);

    assert!(headers.contains_key("traceparent"));
    assert_eq!(
        headers.get("traceparent").unwrap(),
        "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01"
    );

    assert!(headers.contains_key("tracestate"));
    assert_eq!(headers.get("tracestate").unwrap(), "congo=t61rcWkgMzE");
}

#[cfg(feature = "tracing")]
#[test]
fn test_inject_trace_context_without_tracestate() {
    // RED: inject_trace_context doesn't exist yet
    use mizuchi_uploadr::tracing::propagation::TraceContext;

    let context = TraceContext {
        trace_id: "0af7651916cd43dd8448eb211c80319c".to_string(),
        span_id: "b7ad6b7169203331".to_string(),
        trace_flags: 0x00, // not sampled
        tracestate: None,
    };

    let mut headers = HashMap::new();
    inject_trace_context(&context, &mut headers);

    assert!(headers.contains_key("traceparent"));
    assert_eq!(
        headers.get("traceparent").unwrap(),
        "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-00"
    );

    assert!(!headers.contains_key("tracestate"));
}

#[cfg(feature = "tracing")]
#[test]
fn test_roundtrip_extract_inject() {
    // RED: Both functions don't exist yet
    let mut original_headers = HashMap::new();
    original_headers.insert(
        "traceparent".to_string(),
        "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".to_string(),
    );
    original_headers.insert("tracestate".to_string(), "vendor=value".to_string());

    // Extract
    let context = extract_trace_context(&original_headers).unwrap();

    // Inject
    let mut new_headers = HashMap::new();
    inject_trace_context(&context, &mut new_headers);

    // Should match
    assert_eq!(
        original_headers.get("traceparent"),
        new_headers.get("traceparent")
    );
    assert_eq!(
        original_headers.get("tracestate"),
        new_headers.get("tracestate")
    );
}

