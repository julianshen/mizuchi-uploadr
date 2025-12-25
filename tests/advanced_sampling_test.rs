//! Advanced Sampling Strategies Tests
//!
//! Tests for error-based, slow request, and custom rules sampling.
//!
//! TDD Phase: RED - These tests should FAIL initially

#[cfg(feature = "tracing")]
use mizuchi_uploadr::tracing::sampling::{
    AdvancedSampler, ErrorBasedSampler, SamplingDecision, SamplingRule, SlowRequestSampler,
};

#[cfg(feature = "tracing")]
#[test]
fn test_error_based_sampler_samples_errors() {
    // RED: ErrorBasedSampler doesn't exist yet
    let sampler = ErrorBasedSampler::new(0.1); // 10% base rate, 100% for errors

    // Simulate a successful request (should use base rate)
    let decision = sampler.should_sample(false, 100);
    // With 10% base rate, we can't deterministically test, but we can verify it returns a decision
    assert!(matches!(
        decision,
        SamplingDecision::Sample | SamplingDecision::Drop
    ));

    // Simulate an error request (should always sample)
    let decision = sampler.should_sample(true, 100);
    assert_eq!(decision, SamplingDecision::Sample);
}

#[cfg(feature = "tracing")]
#[test]
fn test_slow_request_sampler() {
    // RED: SlowRequestSampler doesn't exist yet
    let sampler = SlowRequestSampler::new(1000, 0.1); // 1000ms threshold, 10% base rate

    // Fast request (100ms) - should use base rate
    let decision = sampler.should_sample(100);
    assert!(matches!(
        decision,
        SamplingDecision::Sample | SamplingDecision::Drop
    ));

    // Slow request (2000ms) - should always sample
    let decision = sampler.should_sample(2000);
    assert_eq!(decision, SamplingDecision::Sample);
}

#[cfg(feature = "tracing")]
#[test]
fn test_slow_request_sampler_at_threshold() {
    // RED: SlowRequestSampler doesn't exist yet
    let sampler = SlowRequestSampler::new(1000, 0.0); // 1000ms threshold, 0% base rate

    // Exactly at threshold - should sample
    let decision = sampler.should_sample(1000);
    assert_eq!(decision, SamplingDecision::Sample);

    // Just below threshold - should not sample (0% base rate)
    let decision = sampler.should_sample(999);
    assert_eq!(decision, SamplingDecision::Drop);
}

#[cfg(feature = "tracing")]
#[test]
fn test_sampling_rule_matches_path() {
    // RED: SamplingRule doesn't exist yet
    let rule = SamplingRule::new()
        .with_path_pattern("/api/critical/*")
        .with_sample_rate(1.0);

    assert!(rule.matches("/api/critical/upload"));
    assert!(rule.matches("/api/critical/delete"));
    assert!(!rule.matches("/api/normal/upload"));
}

#[cfg(feature = "tracing")]
#[test]
fn test_sampling_rule_matches_method() {
    // RED: SamplingRule doesn't exist yet
    let rule = SamplingRule::new()
        .with_method("POST")
        .with_sample_rate(1.0);

    assert!(rule.matches_method("POST"));
    assert!(!rule.matches_method("GET"));
}

#[cfg(feature = "tracing")]
#[test]
fn test_sampling_rule_matches_attribute() {
    // RED: SamplingRule doesn't exist yet
    let rule = SamplingRule::new()
        .with_attribute("user.tier", "premium")
        .with_sample_rate(1.0);

    let mut attributes = std::collections::HashMap::new();
    attributes.insert("user.tier".to_string(), "premium".to_string());

    assert!(rule.matches_attributes(&attributes));

    attributes.insert("user.tier".to_string(), "free".to_string());
    assert!(!rule.matches_attributes(&attributes));
}

#[cfg(feature = "tracing")]
#[test]
fn test_advanced_sampler_with_multiple_rules() {
    // RED: AdvancedSampler doesn't exist yet
    let mut sampler = AdvancedSampler::new(0.1); // 10% base rate

    // Add rule: always sample critical endpoints
    sampler.add_rule(
        SamplingRule::new()
            .with_path_pattern("/api/critical/*")
            .with_sample_rate(1.0),
    );

    // Add rule: always sample premium users
    sampler.add_rule(
        SamplingRule::new()
            .with_attribute("user.tier", "premium")
            .with_sample_rate(1.0),
    );

    // Critical endpoint should always be sampled
    let mut attributes = std::collections::HashMap::new();
    let decision = sampler.should_sample("/api/critical/upload", "POST", &attributes);
    assert_eq!(decision, SamplingDecision::Sample);

    // Premium user should always be sampled
    attributes.insert("user.tier".to_string(), "premium".to_string());
    let decision = sampler.should_sample("/api/normal/upload", "POST", &attributes);
    assert_eq!(decision, SamplingDecision::Sample);

    // Normal request should use base rate
    attributes.clear();
    let decision = sampler.should_sample("/api/normal/upload", "POST", &attributes);
    assert!(matches!(
        decision,
        SamplingDecision::Sample | SamplingDecision::Drop
    ));
}

#[cfg(feature = "tracing")]
#[test]
fn test_advanced_sampler_rule_priority() {
    // RED: AdvancedSampler doesn't exist yet
    let mut sampler = AdvancedSampler::new(0.0); // 0% base rate

    // First rule: sample critical endpoints at 100% (more specific, should be first)
    sampler.add_rule(
        SamplingRule::new()
            .with_path_pattern("/api/critical/*")
            .with_sample_rate(1.0),
    );

    // Second rule: sample all POST requests at 50% (less specific, should be second)
    sampler.add_rule(
        SamplingRule::new()
            .with_method("POST")
            .with_sample_rate(0.5),
    );

    // Critical POST should use the first matching rule (critical at 100%)
    let attributes = std::collections::HashMap::new();
    let decision = sampler.should_sample("/api/critical/upload", "POST", &attributes);
    assert_eq!(decision, SamplingDecision::Sample);

    // Non-critical POST should use the second rule (POST at 50%)
    // Note: This is probabilistic, so we can't assert deterministically
    let decision = sampler.should_sample("/api/normal/upload", "POST", &attributes);
    assert!(matches!(
        decision,
        SamplingDecision::Sample | SamplingDecision::Drop
    ));
}
