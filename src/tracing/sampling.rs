//! Advanced Sampling Strategies
//!
//! Provides sophisticated sampling strategies beyond basic ratio sampling:
//! - Error-based sampling: Always sample traces with errors
//! - Slow request sampling: Sample requests exceeding duration threshold
//! - Custom rules sampling: Sample based on path, method, attributes

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
