//! Advanced Sampling Strategies
//!
//! Provides sophisticated sampling strategies beyond basic ratio sampling:
//! - **Error-based sampling**: Always sample traces with errors
//! - **Slow request sampling**: Sample requests exceeding duration threshold
//! - **Custom rules sampling**: Sample based on path, method, attributes
//!
//! # Overview
//!
//! This module implements advanced sampling strategies that go beyond simple
//! ratio-based sampling. These strategies help you capture important traces
//! while reducing overall sampling overhead.
//!
//! ## Sampling Strategies
//!
//! ### 1. Error-Based Sampling
//!
//! Always samples traces that contain errors, while using a lower base rate
//! for successful requests. This ensures you never miss error traces.
//!
//! ```
//! use mizuchi_uploadr::tracing::sampling::{ErrorBasedSampler, SamplingDecision};
//!
//! let sampler = ErrorBasedSampler::new(0.1); // 10% base rate
//!
//! // Error requests are always sampled
//! assert_eq!(sampler.should_sample(true, 12345), SamplingDecision::Sample);
//!
//! // Successful requests use base rate (10%)
//! let decision = sampler.should_sample(false, 12345);
//! // Decision depends on trace ID hash
//! ```
//!
//! ### 2. Slow Request Sampling
//!
//! Always samples requests that exceed a duration threshold, helping you
//! identify performance issues.
//!
//! ```
//! use mizuchi_uploadr::tracing::sampling::{SlowRequestSampler, SamplingDecision};
//!
//! let sampler = SlowRequestSampler::new(1000, 0.1); // 1000ms threshold, 10% base rate
//!
//! // Slow requests (>= 1000ms) are always sampled
//! assert_eq!(sampler.should_sample(2000), SamplingDecision::Sample);
//!
//! // Fast requests use base rate (10%)
//! let decision = sampler.should_sample(100);
//! ```
//!
//! ### 3. Custom Rules Sampling
//!
//! Define custom sampling rules based on request path, HTTP method, and
//! custom attributes. Rules are evaluated in order (first match wins).
//!
//! ```
//! use mizuchi_uploadr::tracing::sampling::{AdvancedSampler, SamplingRule, SamplingDecision};
//! use std::collections::HashMap;
//!
//! let mut sampler = AdvancedSampler::new(0.1); // 10% base rate
//!
//! // Rule 1: Always sample critical endpoints
//! sampler.add_rule(
//!     SamplingRule::new()
//!         .with_path_pattern("/api/critical/*")
//!         .with_sample_rate(1.0)
//! );
//!
//! // Rule 2: Always sample premium users
//! sampler.add_rule(
//!     SamplingRule::new()
//!         .with_attribute("user.tier", "premium")
//!         .with_sample_rate(1.0)
//! );
//!
//! let attributes = HashMap::new();
//! let decision = sampler.should_sample("/api/critical/upload", "POST", &attributes);
//! assert_eq!(decision, SamplingDecision::Sample);
//! ```
//!
//! ## Best Practices
//!
//! 1. **Order matters**: Place more specific rules first in AdvancedSampler
//! 2. **Error sampling**: Always use ErrorBasedSampler in production
//! 3. **Performance**: Use SlowRequestSampler to catch performance regressions
//! 4. **Critical paths**: Use custom rules to always sample critical endpoints
//! 5. **Base rate**: Set a low base rate (1-10%) to reduce overhead

use std::collections::HashMap;

/// Sampling decision
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SamplingDecision {
    /// Sample this trace
    Sample,
    /// Drop this trace
    Drop,
}

/// Error-based sampler
///
/// Always samples traces with errors, uses base rate for successful requests
pub struct ErrorBasedSampler {
    base_rate: f64,
}

impl ErrorBasedSampler {
    /// Create a new error-based sampler
    ///
    /// # Arguments
    ///
    /// * `base_rate` - Sampling rate for successful requests (0.0 to 1.0)
    pub fn new(base_rate: f64) -> Self {
        Self { base_rate }
    }

    /// Determine if a request should be sampled
    ///
    /// # Arguments
    ///
    /// * `has_error` - Whether the request resulted in an error
    /// * `trace_id` - Trace ID for deterministic sampling
    pub fn should_sample(&self, has_error: bool, trace_id: u64) -> SamplingDecision {
        if has_error {
            // Always sample errors
            return SamplingDecision::Sample;
        }

        // Use deterministic sampling for successful requests
        let threshold = (self.base_rate * u64::MAX as f64) as u64;
        if trace_id <= threshold {
            SamplingDecision::Sample
        } else {
            SamplingDecision::Drop
        }
    }
}

/// Slow request sampler
///
/// Always samples requests exceeding a duration threshold
pub struct SlowRequestSampler {
    threshold_ms: u64,
    base_rate: f64,
}

impl SlowRequestSampler {
    /// Create a new slow request sampler
    ///
    /// # Arguments
    ///
    /// * `threshold_ms` - Duration threshold in milliseconds
    /// * `base_rate` - Sampling rate for fast requests (0.0 to 1.0)
    pub fn new(threshold_ms: u64, base_rate: f64) -> Self {
        Self {
            threshold_ms,
            base_rate,
        }
    }

    /// Determine if a request should be sampled
    ///
    /// # Arguments
    ///
    /// * `duration_ms` - Request duration in milliseconds
    pub fn should_sample(&self, duration_ms: u64) -> SamplingDecision {
        if duration_ms >= self.threshold_ms {
            // Always sample slow requests
            return SamplingDecision::Sample;
        }

        // Use base rate for fast requests
        if self.base_rate >= 1.0 {
            SamplingDecision::Sample
        } else if self.base_rate <= 0.0 {
            SamplingDecision::Drop
        } else {
            // For simplicity, use a deterministic approach based on duration
            let threshold = (self.base_rate * u64::MAX as f64) as u64;
            if duration_ms <= threshold {
                SamplingDecision::Sample
            } else {
                SamplingDecision::Drop
            }
        }
    }
}

/// Sampling rule
///
/// Defines conditions for sampling based on path, method, and attributes
#[derive(Debug, Clone)]
pub struct SamplingRule {
    path_pattern: Option<String>,
    method: Option<String>,
    attributes: HashMap<String, String>,
    sample_rate: f64,
}

impl SamplingRule {
    /// Create a new sampling rule
    pub fn new() -> Self {
        Self {
            path_pattern: None,
            method: None,
            attributes: HashMap::new(),
            sample_rate: 1.0,
        }
    }

    /// Set path pattern for this rule
    pub fn with_path_pattern(mut self, pattern: &str) -> Self {
        self.path_pattern = Some(pattern.to_string());
        self
    }

    /// Set HTTP method for this rule
    pub fn with_method(mut self, method: &str) -> Self {
        self.method = Some(method.to_string());
        self
    }

    /// Add attribute condition for this rule
    pub fn with_attribute(mut self, key: &str, value: &str) -> Self {
        self.attributes.insert(key.to_string(), value.to_string());
        self
    }

    /// Set sample rate for this rule
    pub fn with_sample_rate(mut self, rate: f64) -> Self {
        self.sample_rate = rate;
        self
    }

    /// Check if path matches the pattern
    pub fn matches(&self, path: &str) -> bool {
        if let Some(ref pattern) = self.path_pattern {
            // Simple wildcard matching
            if pattern.ends_with("/*") {
                let prefix = &pattern[..pattern.len() - 2];
                return path.starts_with(prefix);
            }
            return path == pattern;
        }
        true
    }

    /// Check if method matches
    pub fn matches_method(&self, method: &str) -> bool {
        if let Some(ref m) = self.method {
            return m == method;
        }
        true
    }

    /// Check if attributes match
    pub fn matches_attributes(&self, attributes: &HashMap<String, String>) -> bool {
        for (key, value) in &self.attributes {
            if attributes.get(key) != Some(value) {
                return false;
            }
        }
        true
    }

    /// Get sample rate for this rule
    pub fn sample_rate(&self) -> f64 {
        self.sample_rate
    }
}

impl Default for SamplingRule {
    fn default() -> Self {
        Self::new()
    }
}

/// Advanced sampler with custom rules
///
/// Evaluates multiple sampling rules and uses base rate as fallback
pub struct AdvancedSampler {
    base_rate: f64,
    rules: Vec<SamplingRule>,
}

impl AdvancedSampler {
    /// Create a new advanced sampler
    ///
    /// # Arguments
    ///
    /// * `base_rate` - Default sampling rate when no rules match (0.0 to 1.0)
    pub fn new(base_rate: f64) -> Self {
        Self {
            base_rate,
            rules: Vec::new(),
        }
    }

    /// Add a sampling rule
    pub fn add_rule(&mut self, rule: SamplingRule) {
        self.rules.push(rule);
    }

    /// Determine if a request should be sampled
    ///
    /// # Arguments
    ///
    /// * `path` - Request path
    /// * `method` - HTTP method
    /// * `attributes` - Request attributes
    pub fn should_sample(
        &self,
        path: &str,
        method: &str,
        attributes: &HashMap<String, String>,
    ) -> SamplingDecision {
        // Check rules in order (first match wins)
        for rule in &self.rules {
            if rule.matches(path)
                && rule.matches_method(method)
                && rule.matches_attributes(attributes)
            {
                let rate = rule.sample_rate();
                if rate >= 1.0 {
                    return SamplingDecision::Sample;
                } else if rate <= 0.0 {
                    return SamplingDecision::Drop;
                }
                // For simplicity, use deterministic sampling based on path hash
                let hash = path
                    .bytes()
                    .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
                let threshold = (rate * u64::MAX as f64) as u64;
                return if hash <= threshold {
                    SamplingDecision::Sample
                } else {
                    SamplingDecision::Drop
                };
            }
        }

        // No rules matched, use base rate
        if self.base_rate >= 1.0 {
            SamplingDecision::Sample
        } else if self.base_rate <= 0.0 {
            SamplingDecision::Drop
        } else {
            // Use deterministic sampling based on path hash
            let hash = path
                .bytes()
                .fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
            let threshold = (self.base_rate * u64::MAX as f64) as u64;
            if hash <= threshold {
                SamplingDecision::Sample
            } else {
                SamplingDecision::Drop
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_sampler_always_samples_errors() {
        let sampler = ErrorBasedSampler::new(0.0); // 0% base rate

        // Errors should always be sampled regardless of base rate
        assert_eq!(sampler.should_sample(true, 0), SamplingDecision::Sample);
        assert_eq!(
            sampler.should_sample(true, u64::MAX),
            SamplingDecision::Sample
        );
    }

    #[test]
    fn test_error_sampler_respects_base_rate() {
        let sampler = ErrorBasedSampler::new(1.0); // 100% base rate

        // Success should be sampled with 100% rate
        assert_eq!(sampler.should_sample(false, 0), SamplingDecision::Sample);
        assert_eq!(
            sampler.should_sample(false, u64::MAX),
            SamplingDecision::Sample
        );
    }

    #[test]
    fn test_slow_sampler_always_samples_slow_requests() {
        let sampler = SlowRequestSampler::new(1000, 0.0); // 1000ms threshold, 0% base rate

        // Slow requests should always be sampled
        assert_eq!(sampler.should_sample(1000), SamplingDecision::Sample);
        assert_eq!(sampler.should_sample(2000), SamplingDecision::Sample);
        assert_eq!(sampler.should_sample(u64::MAX), SamplingDecision::Sample);
    }

    #[test]
    fn test_slow_sampler_respects_base_rate_for_fast_requests() {
        let sampler = SlowRequestSampler::new(1000, 1.0); // 1000ms threshold, 100% base rate

        // Fast requests should use base rate
        assert_eq!(sampler.should_sample(0), SamplingDecision::Sample);
        assert_eq!(sampler.should_sample(999), SamplingDecision::Sample);
    }

    #[test]
    fn test_sampling_rule_wildcard_matching() {
        let rule = SamplingRule::new()
            .with_path_pattern("/api/v1/*")
            .with_sample_rate(1.0);

        assert!(rule.matches("/api/v1/users"));
        assert!(rule.matches("/api/v1/posts"));
        assert!(rule.matches("/api/v1/"));
        assert!(!rule.matches("/api/v2/users"));
        assert!(!rule.matches("/api/users"));
    }

    #[test]
    fn test_sampling_rule_exact_matching() {
        let rule = SamplingRule::new()
            .with_path_pattern("/api/health")
            .with_sample_rate(1.0);

        assert!(rule.matches("/api/health"));
        assert!(!rule.matches("/api/health/check"));
        assert!(!rule.matches("/api/healthz"));
    }

    #[test]
    fn test_sampling_rule_multiple_attributes() {
        let rule = SamplingRule::new()
            .with_attribute("user.tier", "premium")
            .with_attribute("region", "us-east-1")
            .with_sample_rate(1.0);

        let mut attrs = HashMap::new();
        attrs.insert("user.tier".to_string(), "premium".to_string());
        attrs.insert("region".to_string(), "us-east-1".to_string());
        assert!(rule.matches_attributes(&attrs));

        // Missing one attribute
        attrs.remove("region");
        assert!(!rule.matches_attributes(&attrs));
    }

    #[test]
    fn test_advanced_sampler_no_rules() {
        let sampler = AdvancedSampler::new(1.0); // 100% base rate
        let attrs = HashMap::new();

        // Should use base rate when no rules match
        assert_eq!(
            sampler.should_sample("/any/path", "GET", &attrs),
            SamplingDecision::Sample
        );
    }

    #[test]
    fn test_advanced_sampler_first_match_wins() {
        let mut sampler = AdvancedSampler::new(0.0);

        // First rule: sample /api/* at 100%
        sampler.add_rule(
            SamplingRule::new()
                .with_path_pattern("/api/*")
                .with_sample_rate(1.0),
        );

        // Second rule: sample /api/health at 0% (should not be used)
        sampler.add_rule(
            SamplingRule::new()
                .with_path_pattern("/api/health")
                .with_sample_rate(0.0),
        );

        let attrs = HashMap::new();
        // First rule should match and sample
        assert_eq!(
            sampler.should_sample("/api/health", "GET", &attrs),
            SamplingDecision::Sample
        );
    }
}
