//! Tracing Performance Benchmarks
//!
//! Measures the performance overhead of distributed tracing to ensure
//! it stays below 5% of request latency.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use mizuchi_uploadr::tracing::propagation::{
    extract_trace_context, inject_trace_context, TraceContext,
};
use mizuchi_uploadr::tracing::sampling::{
    AdvancedSampler, ErrorBasedSampler, SamplingDecision, SamplingRule, SlowRequestSampler,
};
use std::collections::HashMap;

/// Benchmark trace context extraction from headers
fn bench_extract_trace_context(c: &mut Criterion) {
    let mut headers = HashMap::new();
    headers.insert(
        "traceparent".to_string(),
        "00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01".to_string(),
    );
    headers.insert("tracestate".to_string(), "congo=t61rcWkgMzE".to_string());

    c.bench_function("extract_trace_context", |b| {
        b.iter(|| {
            let _ = black_box(extract_trace_context(&headers));
        });
    });
}

/// Benchmark trace context injection into headers
fn bench_inject_trace_context(c: &mut Criterion) {
    let context = TraceContext {
        trace_id: "0af7651916cd43dd8448eb211c80319c".to_string(),
        span_id: "b7ad6b7169203331".to_string(),
        trace_flags: 1,
        tracestate: Some("congo=t61rcWkgMzE".to_string()),
    };

    c.bench_function("inject_trace_context", |b| {
        b.iter(|| {
            let mut headers = HashMap::new();
            inject_trace_context(black_box(&context), &mut headers);
            black_box(headers);
        });
    });
}

/// Benchmark error-based sampler decision
fn bench_error_sampler(c: &mut Criterion) {
    let sampler = ErrorBasedSampler::new(0.1);

    let mut group = c.benchmark_group("error_sampler");

    group.bench_function("sample_error", |b| {
        b.iter(|| {
            let decision = sampler.should_sample(black_box(true), black_box(12345));
            black_box(decision);
        });
    });

    group.bench_function("sample_success", |b| {
        b.iter(|| {
            let decision = sampler.should_sample(black_box(false), black_box(12345));
            black_box(decision);
        });
    });

    group.finish();
}

/// Benchmark slow request sampler decision
fn bench_slow_sampler(c: &mut Criterion) {
    let sampler = SlowRequestSampler::new(1000, 0.1);

    let mut group = c.benchmark_group("slow_sampler");

    group.bench_function("sample_slow", |b| {
        b.iter(|| {
            let decision = sampler.should_sample(black_box(2000));
            black_box(decision);
        });
    });

    group.bench_function("sample_fast", |b| {
        b.iter(|| {
            let decision = sampler.should_sample(black_box(100));
            black_box(decision);
        });
    });

    group.finish();
}

/// Benchmark advanced sampler with multiple rules
fn bench_advanced_sampler(c: &mut Criterion) {
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

    let mut group = c.benchmark_group("advanced_sampler");

    // Benchmark matching first rule
    group.bench_function("match_first_rule", |b| {
        let attrs = HashMap::new();
        b.iter(|| {
            let decision = sampler.should_sample(
                black_box("/api/critical/upload"),
                black_box("POST"),
                black_box(&attrs),
            );
            black_box(decision);
        });
    });

    // Benchmark matching second rule
    group.bench_function("match_second_rule", |b| {
        let attrs = HashMap::new();
        b.iter(|| {
            let decision = sampler.should_sample(
                black_box("/api/normal/upload"),
                black_box("POST"),
                black_box(&attrs),
            );
            black_box(decision);
        });
    });

    // Benchmark matching third rule
    group.bench_function("match_third_rule", |b| {
        let mut attrs = HashMap::new();
        attrs.insert("user.tier".to_string(), "premium".to_string());
        b.iter(|| {
            let decision = sampler.should_sample(
                black_box("/api/normal/upload"),
                black_box("GET"),
                black_box(&attrs),
            );
            black_box(decision);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_extract_trace_context,
    bench_inject_trace_context,
    bench_error_sampler,
    bench_slow_sampler,
    bench_advanced_sampler,
);
criterion_main!(benches);
