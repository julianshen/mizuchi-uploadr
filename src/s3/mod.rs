//! S3 Client module
//!
//! Provides S3 client with SigV4 signing.

use thiserror::Error;

/// S3 client errors
#[derive(Error, Debug)]
pub enum S3ClientError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Request error: {0}")]
    RequestError(String),

    #[error("Response error: {0}")]
    ResponseError(String),

    #[error("Signing error: {0}")]
    SigningError(String),
}

/// S3 Client configuration
#[derive(Debug, Clone)]
pub struct S3ClientConfig {
    pub bucket: String,
    pub region: String,
    pub endpoint: Option<String>,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
}

/// S3 Client
pub struct S3Client {
    config: S3ClientConfig,
    http_client: reqwest::Client,
}

impl S3Client {
    /// Create a new S3 client
    pub fn new(config: S3ClientConfig) -> Result<Self, S3ClientError> {
        let http_client = reqwest::Client::builder()
            .build()
            .map_err(|e| S3ClientError::ConfigError(e.to_string()))?;

        Ok(Self {
            config,
            http_client,
        })
    }

    /// Get the bucket name
    pub fn bucket(&self) -> &str {
        &self.config.bucket
    }

    /// Get the region
    pub fn region(&self) -> &str {
        &self.config.region
    }

    /// Get the endpoint URL
    pub fn endpoint(&self) -> String {
        self.config.endpoint.clone().unwrap_or_else(|| {
            format!("https://s3.{}.amazonaws.com", self.config.region)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_s3_client_creation() {
        let config = S3ClientConfig {
            bucket: "test-bucket".into(),
            region: "us-east-1".into(),
            endpoint: None,
            access_key: None,
            secret_key: None,
        };

        let client = S3Client::new(config).unwrap();
        assert_eq!(client.bucket(), "test-bucket");
        assert_eq!(client.region(), "us-east-1");
    }

    #[test]
    fn test_default_endpoint() {
        let config = S3ClientConfig {
            bucket: "test-bucket".into(),
            region: "us-west-2".into(),
            endpoint: None,
            access_key: None,
            secret_key: None,
        };

        let client = S3Client::new(config).unwrap();
        assert_eq!(client.endpoint(), "https://s3.us-west-2.amazonaws.com");
    }

    #[test]
    fn test_custom_endpoint() {
        let config = S3ClientConfig {
            bucket: "test-bucket".into(),
            region: "us-east-1".into(),
            endpoint: Some("http://localhost:9000".into()),
            access_key: None,
            secret_key: None,
        };

        let client = S3Client::new(config).unwrap();
        assert_eq!(client.endpoint(), "http://localhost:9000");
    }
}
