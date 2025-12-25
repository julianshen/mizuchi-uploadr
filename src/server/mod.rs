//! HTTP server module
//!
//! Handles incoming HTTP requests and routes them to appropriate handlers.

use crate::config::Config;
use std::net::SocketAddr;
use thiserror::Error;
use tracing::info;

/// Server errors
#[derive(Error, Debug)]
pub enum ServerError {
    #[error("Failed to bind to address: {0}")]
    BindError(String),

    #[error("Server error: {0}")]
    RuntimeError(String),
}

/// HTTP Server
pub struct Server {
    config: Config,
    addr: SocketAddr,
}

impl Server {
    /// Create a new server instance
    pub fn new(config: Config) -> Result<Self, ServerError> {
        let addr: SocketAddr = config
            .server
            .address
            .parse()
            .map_err(|e| ServerError::BindError(format!("{}", e)))?;

        Ok(Self { config, addr })
    }

    /// Run the server
    pub async fn run(&self) -> Result<(), ServerError> {
        info!("Starting server on {}", self.addr);
        info!(
            "Zero-copy: {}",
            if self.config.server.zero_copy.enabled && crate::zero_copy_available() {
                "enabled"
            } else {
                "disabled"
            }
        );

        // TODO: Implement actual server logic
        // This is a placeholder for the TDD approach
        // RED: Tests will be written first
        // GREEN: Implementation will follow

        tokio::signal::ctrl_c()
            .await
            .map_err(|e| ServerError::RuntimeError(e.to_string()))?;

        info!("Shutting down server");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{BucketConfig, MetricsConfig, S3Config, ServerConfig, ZeroCopyConfig};

    fn test_config() -> Config {
        Config {
            server: ServerConfig {
                address: "127.0.0.1:0".into(),
                zero_copy: ZeroCopyConfig::default(),
            },
            buckets: vec![BucketConfig {
                name: "test".into(),
                path_prefix: "/test".into(),
                s3: S3Config {
                    bucket: "test-bucket".into(),
                    region: "us-east-1".into(),
                    endpoint: None,
                    access_key: None,
                    secret_key: None,
                },
                auth: Default::default(),
                upload: Default::default(),
            }],
            metrics: MetricsConfig::default(),
            tracing: None,
        }
    }

    #[test]
    fn test_server_new() {
        let config = test_config();
        let server = Server::new(config);
        assert!(server.is_ok());
    }

    #[test]
    fn test_server_invalid_address() {
        let mut config = test_config();
        config.server.address = "invalid".into();
        let server = Server::new(config);
        assert!(server.is_err());
    }
}
