//! Configuration module for Mizuchi Uploadr
//!
//! Handles loading and parsing of YAML configuration files.

use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

mod loader;

pub use loader::ConfigLoader;

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
        };

        assert!(config.validate().is_err());
    }
}
