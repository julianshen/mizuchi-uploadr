//! Configuration module for Mizuchi Uploadr
//!
//! Handles loading and parsing of YAML configuration files with support for
//! environment variable expansion and comprehensive validation.

use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

mod loader;

pub use loader::ConfigLoader;

// ============================================================================
// Environment Variable Expansion
// ============================================================================

/// Expand environment variables in a string.
///
/// Supports `${VAR_NAME}` syntax where VAR_NAME must start with a letter or
/// underscore and contain only uppercase letters, digits, and underscores.
///
/// # Examples
///
/// ```ignore
/// std::env::set_var("MY_VAR", "value");
/// let result = expand_env_vars("prefix-${MY_VAR}-suffix");
/// assert_eq!(result, "prefix-value-suffix");
/// ```
fn expand_env_vars(s: &str) -> String {
    let mut result = s.to_string();
    let re = regex_lite::Regex::new(r"\$\{([A-Z_][A-Z0-9_]*)\}").unwrap();

    for cap in re.captures_iter(s) {
        let var_name = &cap[1];
        if let Ok(value) = std::env::var(var_name) {
            result = result.replace(&format!("${{{}}}", var_name), &value);
        }
    }

    result
}

/// Custom deserializer for strings with environment variable expansion.
///
/// This is used with serde's `deserialize_with` attribute to automatically
/// expand environment variables when deserializing configuration values.
fn deserialize_with_env<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: serde::de::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(expand_env_vars(&s))
}

// ============================================================================
// Validation Helpers
// ============================================================================

/// Validate that a URL starts with http:// or https://
fn is_valid_http_url(url: &str) -> bool {
    url.starts_with("http://") || url.starts_with("https://")
}

/// Configuration errors
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Failed to read config file: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Failed to parse config: {0}")]
    ParseError(#[from] serde_yaml::Error),

    #[error("Invalid configuration: {0}")]
    ValidationError(String),
}

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub buckets: Vec<BucketConfig>,
    #[serde(default)]
    pub metrics: MetricsConfig,
    #[serde(default)]
    pub tracing: Option<TracingConfig>,
}

impl Config {
    /// Load configuration from a file
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        ConfigLoader::load(path)
    }

    /// Validate the configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.buckets.is_empty() {
            return Err(ConfigError::ValidationError(
                "At least one bucket must be configured".into(),
            ));
        }

        for bucket in &self.buckets {
            if bucket.path_prefix.is_empty() {
                return Err(ConfigError::ValidationError(format!(
                    "Bucket '{}' has empty path_prefix",
                    bucket.name
                )));
            }
        }

        // Validate tracing config if present
        if let Some(ref tracing) = self.tracing {
            if tracing.enabled && !is_valid_http_url(&tracing.otlp.endpoint) {
                return Err(ConfigError::ValidationError(
                    "Invalid OTLP endpoint: must start with http:// or https://".into(),
                ));
            }
        }

        Ok(())
    }
}

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub address: String,
    #[serde(default)]
    pub zero_copy: ZeroCopyConfig,
}

/// Zero-copy transfer configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZeroCopyConfig {
    #[serde(default = "default_zero_copy_enabled")]
    pub enabled: bool,
    #[serde(default = "default_pipe_buffer_size")]
    pub pipe_buffer_size: usize,
}

impl Default for ZeroCopyConfig {
    fn default() -> Self {
        Self {
            enabled: default_zero_copy_enabled(),
            pipe_buffer_size: default_pipe_buffer_size(),
        }
    }
}

fn default_zero_copy_enabled() -> bool {
    true
}

fn default_pipe_buffer_size() -> usize {
    1048576 // 1MB
}

/// Bucket configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BucketConfig {
    pub name: String,
    pub path_prefix: String,
    pub s3: S3Config,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub upload: UploadConfig,
}

/// S3 backend configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct S3Config {
    pub bucket: String,
    pub region: String,
    #[serde(default)]
    pub endpoint: Option<String>,
    #[serde(default)]
    pub access_key: Option<String>,
    #[serde(default)]
    pub secret_key: Option<String>,
}

/// Authentication configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AuthConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub jwt: Option<JwtConfig>,
    #[serde(default)]
    pub sigv4: Option<SigV4Config>,
}

/// JWT configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtConfig {
    pub secret: Option<String>,
    pub algorithm: String,
    #[serde(default)]
    pub jwks_url: Option<String>,
    #[serde(default)]
    pub token_sources: Vec<TokenSource>,
}

/// Token source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum TokenSource {
    #[serde(rename = "bearer")]
    Bearer,
    #[serde(rename = "query")]
    Query { name: String },
    #[serde(rename = "header")]
    Header { name: String },
}

/// SigV4 configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SigV4Config {
    pub service: String,
    pub region: String,
}

/// Upload configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadConfig {
    #[serde(default = "default_multipart_threshold")]
    pub multipart_threshold: usize,
    #[serde(default = "default_part_size")]
    pub part_size: usize,
    #[serde(default = "default_concurrent_parts")]
    pub concurrent_parts: usize,
}

impl Default for UploadConfig {
    fn default() -> Self {
        Self {
            multipart_threshold: default_multipart_threshold(),
            part_size: default_part_size(),
            concurrent_parts: default_concurrent_parts(),
        }
    }
}

fn default_multipart_threshold() -> usize {
    52428800 // 50MB
}

fn default_part_size() -> usize {
    104857600 // 100MB
}

fn default_concurrent_parts() -> usize {
    4
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    #[serde(default = "default_metrics_enabled")]
    pub enabled: bool,
    #[serde(default = "default_metrics_port")]
    pub port: u16,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        Self {
            enabled: default_metrics_enabled(),
            port: default_metrics_port(),
        }
    }
}

fn default_metrics_enabled() -> bool {
    true
}

fn default_metrics_port() -> u16 {
    9090
}

// ============================================================================
// Tracing Configuration
// ============================================================================

/// OpenTelemetry distributed tracing configuration.
///
/// Enables distributed tracing with OTLP (OpenTelemetry Protocol) export to
/// backends like Jaeger, Tempo, or any OTLP-compatible collector.
///
/// # Example
///
/// ```yaml
/// tracing:
///   enabled: true
///   service_name: "mizuchi-uploadr"
///   otlp:
///     endpoint: "http://localhost:4317"
///     protocol: "grpc"
///   sampling:
///     strategy: "always"
///     ratio: 1.0
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TracingConfig {
    /// Enable or disable tracing. Default: false
    #[serde(default)]
    pub enabled: bool,

    /// Service name for trace identification. Supports ${VAR} expansion.
    #[serde(deserialize_with = "deserialize_with_env")]
    pub service_name: String,

    /// OTLP exporter configuration
    #[serde(default)]
    pub otlp: OtlpConfig,

    /// Trace sampling configuration
    #[serde(default)]
    pub sampling: SamplingConfig,

    /// Batch span processor configuration
    #[serde(default)]
    pub batch: BatchConfig,
}

/// OTLP (OpenTelemetry Protocol) exporter configuration.
///
/// Configures how traces are exported to an OTLP-compatible backend.
///
/// # Supported Protocols
/// - `grpc` - gRPC protocol (default, recommended)
/// - `http/protobuf` - HTTP with protobuf encoding
///
/// # Example
///
/// ```yaml
/// otlp:
///   endpoint: "${OTLP_ENDPOINT}"  # Supports env vars
///   protocol: "grpc"
///   timeout_seconds: 10
///   compression: "gzip"  # Optional: gzip, none
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtlpConfig {
    /// OTLP collector endpoint URL. Supports ${VAR} expansion.
    /// Must start with http:// or https://
    #[serde(deserialize_with = "deserialize_with_env")]
    pub endpoint: String,

    /// Protocol to use: "grpc" or "http/protobuf". Default: "grpc"
    #[serde(default = "default_otlp_protocol")]
    pub protocol: String,

    /// Timeout for OTLP export in seconds. Default: 10
    #[serde(default = "default_otlp_timeout")]
    pub timeout_seconds: u64,

    /// Optional compression: "gzip" or "none". Default: none
    #[serde(default)]
    pub compression: Option<String>,
}

impl Default for OtlpConfig {
    fn default() -> Self {
        Self {
            endpoint: String::new(),
            protocol: default_otlp_protocol(),
            timeout_seconds: default_otlp_timeout(),
            compression: None,
        }
    }
}

fn default_otlp_protocol() -> String {
    "grpc".to_string()
}

fn default_otlp_timeout() -> u64 {
    10
}

/// Trace sampling configuration.
///
/// Controls which traces are recorded and exported to reduce overhead
/// and storage costs in high-traffic environments.
///
/// # Sampling Strategies
/// - `always` - Sample all traces (100%, default for development)
/// - `never` - Sample no traces (0%, useful for disabling)
/// - `ratio` - Sample a percentage of traces based on `ratio` field
/// - `parent_based` - Respect parent span's sampling decision
///
/// # Example
///
/// ```yaml
/// sampling:
///   strategy: "ratio"
///   ratio: 0.1  # Sample 10% of traces
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SamplingConfig {
    /// Sampling strategy. Default: "always"
    #[serde(default = "default_sampling_strategy")]
    pub strategy: String,

    /// Sampling ratio (0.0 to 1.0). Only used with "ratio" strategy. Default: 1.0
    #[serde(default = "default_sampling_ratio")]
    pub ratio: f64,
}

impl Default for SamplingConfig {
    fn default() -> Self {
        Self {
            strategy: default_sampling_strategy(),
            ratio: default_sampling_ratio(),
        }
    }
}

fn default_sampling_strategy() -> String {
    "always".to_string()
}

fn default_sampling_ratio() -> f64 {
    1.0
}

/// Batch span processor configuration.
///
/// Controls how spans are batched before export to reduce network overhead
/// and improve performance. The processor exports spans when either the
/// queue size or scheduled delay threshold is reached.
///
/// # Performance Tuning
/// - Increase `max_queue_size` for high-throughput scenarios
/// - Decrease `scheduled_delay_millis` for lower latency (more frequent exports)
/// - Adjust `max_export_batch_size` based on OTLP backend limits
///
/// # Example
///
/// ```yaml
/// batch:
///   max_queue_size: 2048
///   scheduled_delay_millis: 5000  # Export every 5 seconds
///   max_export_batch_size: 512
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchConfig {
    /// Maximum number of spans to queue before forcing export. Default: 2048
    #[serde(default = "default_max_queue_size")]
    pub max_queue_size: usize,

    /// Delay in milliseconds between scheduled exports. Default: 5000 (5 seconds)
    #[serde(default = "default_scheduled_delay")]
    pub scheduled_delay_millis: u64,

    /// Maximum number of spans per export batch. Default: 512
    #[serde(default = "default_max_export_batch_size")]
    pub max_export_batch_size: usize,
}

impl Default for BatchConfig {
    fn default() -> Self {
        Self {
            max_queue_size: default_max_queue_size(),
            scheduled_delay_millis: default_scheduled_delay(),
            max_export_batch_size: default_max_export_batch_size(),
        }
    }
}

fn default_max_queue_size() -> usize {
    2048
}

fn default_scheduled_delay() -> u64 {
    5000
}

fn default_max_export_batch_size() -> usize {
    512
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_zero_copy_config() {
        let config = ZeroCopyConfig::default();
        assert!(config.enabled);
        assert_eq!(config.pipe_buffer_size, 1048576);
    }

    #[test]
    fn test_config_validation_empty_buckets() {
        let config = Config {
            server: ServerConfig {
                address: "0.0.0.0:8080".into(),
                zero_copy: ZeroCopyConfig::default(),
            },
            buckets: vec![],
            metrics: MetricsConfig::default(),
            tracing: None,
        };

        assert!(config.validate().is_err());
    }
}
