//! Tracing Performance Tests
//!
//! Validates that tracing overhead stays below acceptable thresholds.

#[cfg(feature = "tracing")]
use mizuchi_uploadr::tracing::propagation::{
    extract_trace_context, inject_trace_context, TraceContext,
};
#[cfg(feature = "tracing")]
use mizuchi_uploadr::tracing::sampling::{
    AdvancedSampler, ErrorBasedSampler, SamplingRule, SlowRequestSampler,
};
#[cfg(feature = "tracing")]
use std::collections::HashMap;
#[cfg(feature = "tracing")]
use std::time::Instant;

/// Test that trace context extraction is fast (<3μs)
#[cfg(feature = "tracing")]
#[test]
fn test_extract_trace_context_performance() {
    let mut headers = HashMap::new();
    headers.insert(
        "traceparent".to_string(),
        "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".to_string(),
    );
    headers.insert("tracestate".to_string(), "congo=t61rcWkgMzE".to_string());

    // Warm up
    for _ in 0..100 {
        let _ = extract_trace_context(&headers);
    }

    // Measure
    let iterations = 10000u128;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = extract_trace_context(&headers);
    }
    let elapsed = start.elapsed();

    let avg_nanos = elapsed.as_nanos() / iterations as u128;
    println!("Average extract time: {}ns", avg_nanos);

    // Should be less than 3μs (3000ns) - realistic target for HashMap lookup + parsing
    assert!(
        avg_nanos < 3000,
        "Extract trace context too slow: {}ns > 3000ns",
        avg_nanos
    );
}

/// Test that trace context injection is fast (<3μs)
#[cfg(feature = "tracing")]
#[test]
fn test_inject_trace_context_performance() {
    let context = TraceContext {
        trace_id: "0af7651916cd43dd8448eb211c80319c".to_string(),
        span_id: "b7ad6b7169203331".to_string(),
        trace_flags: 1,
        tracestate: Some("congo=t61rcWkgMzE".to_string()),
    };

    // Warm up
    for _ in 0..100 {
        let mut headers = HashMap::new();
        inject_trace_context(&context, &mut headers);
    }

    // Measure
    let iterations = 10000u128;
    let start = Instant::now();
    for _ in 0..iterations {
        let mut headers = HashMap::new();
        inject_trace_context(&context, &mut headers);
    }
    let elapsed = start.elapsed();

    let avg_nanos = elapsed.as_nanos() / iterations;
    println!("Average inject time: {}ns", avg_nanos);

    // Should be less than 3μs (3000ns) - realistic target for string formatting + HashMap insert
    assert!(
        avg_nanos < 3000,
        "Inject trace context too slow: {}ns > 3000ns",
        avg_nanos
    );
}

/// Test that error sampler decision is fast (<100ns)
#[cfg(feature = "tracing")]
#[test]
fn test_error_sampler_performance() {
    let sampler = ErrorBasedSampler::new(0.1);

    // Warm up
    for i in 0..100 {
        let _ = sampler.should_sample(i % 2 == 0, i as u64);
    }

    // Measure
    let iterations = 100000u128;
    let start = Instant::now();
    for i in 0..iterations {
        let _ = sampler.should_sample(i % 2 == 0, i as u64);
    }
    let elapsed = start.elapsed();

    let avg_nanos = elapsed.as_nanos() / iterations;
    println!("Average error sampler time: {}ns", avg_nanos);

    // Should be less than 100ns
    assert!(
        avg_nanos < 100,
        "Error sampler too slow: {}ns > 100ns",
        avg_nanos
    );
}

/// Test that slow request sampler decision is fast (<100ns)
#[cfg(feature = "tracing")]
#[test]
fn test_slow_sampler_performance() {
    let sampler = SlowRequestSampler::new(1000, 0.1);

    // Warm up
    for i in 0..100 {
        let _ = sampler.should_sample(i);
    }

    // Measure
    let iterations = 100000u128;
    let start = Instant::now();
    for i in 0..iterations {
        let _ = sampler.should_sample(i as u64);
    }
    let elapsed = start.elapsed();

    let avg_nanos = elapsed.as_nanos() / iterations;
    println!("Average slow sampler time: {}ns", avg_nanos);

    // Should be less than 100ns
    assert!(
        avg_nanos < 100,
        "Slow sampler too slow: {}ns > 100ns",
        avg_nanos
    );
}

/// Test that advanced sampler with multiple rules is fast (<500ns)
#[cfg(feature = "tracing")]
#[test]
fn test_advanced_sampler_performance() {
    let mut sampler = AdvancedSampler::new(0.1);

    // Add multiple rules
    sampler.add_rule(
        SamplingRule::new()
            .with_path_pattern("/api/critical/*")
            .with_sample_rate(1.0),
    );
    sampler.add_rule(
        SamplingRule::new()
            .with_method("POST")
            .with_sample_rate(0.5),
    );
    sampler.add_rule(
        SamplingRule::new()
            .with_attribute("user.tier", "premium")
            .with_sample_rate(1.0),
    );

    let attrs = HashMap::new();

    // Warm up
    for _ in 0..100 {
        let _ = sampler.should_sample("/api/critical/upload", "POST", &attrs);
    }

    // Measure
    let iterations = 10000u128;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = sampler.should_sample("/api/critical/upload", "POST", &attrs);
    }
    let elapsed = start.elapsed();

    let avg_nanos = elapsed.as_nanos() / iterations;
    println!("Average advanced sampler time: {}ns", avg_nanos);

    // Should be less than 500ns
    assert!(
        avg_nanos < 500,
        "Advanced sampler too slow: {}ns > 500ns",
        avg_nanos
    );
}
