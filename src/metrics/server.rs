//! Prometheus Metrics HTTP Server
//!
//! Provides an HTTP endpoint for Prometheus to scrape metrics.
//!
//! # Features
//!
//! - `/metrics` - Prometheus text format metrics
//! - `/health` - Health check endpoint for Kubernetes
//! - Graceful shutdown support
//! - Builder pattern for configuration
//!
//! # Example
//!
//! ```no_run
//! use mizuchi_uploadr::metrics::server::{MetricsServer, MetricsServerConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Direct configuration
//!     let config = MetricsServerConfig {
//!         address: "127.0.0.1:9090".to_string(),
//!     };
//!     let mut server = MetricsServer::new(config);
//!     let addr = server.start().await?;
//!     println!("Metrics server listening on {}", addr);
//!
//!     // Or use builder pattern
//!     let mut server = MetricsServer::builder()
//!         .address("127.0.0.1:9090")
//!         .build()?;
//!     server.start().await?;
//!
//!     Ok(())
//! }
//! ```

use bytes::Bytes;
use http_body_util::Full;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Method, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use prometheus::{Encoder, TextEncoder};
use std::convert::Infallible;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

/// Configuration for the metrics server
#[derive(Debug, Clone)]
pub struct MetricsServerConfig {
    /// Address to bind to (e.g., "127.0.0.1:9090" or "0.0.0.0:9090")
    pub address: String,
}

/// Builder for MetricsServer
///
/// Provides a fluent API for constructing a MetricsServer.
#[derive(Default)]
pub struct MetricsServerBuilder {
    address: Option<String>,
}

impl MetricsServerBuilder {
    /// Set the server address
    ///
    /// # Examples
    ///
    /// ```
    /// use mizuchi_uploadr::metrics::server::MetricsServer;
    ///
    /// let server = MetricsServer::builder()
    ///     .address("127.0.0.1:9090")
    ///     .build();
    /// ```
    pub fn address(mut self, address: &str) -> Self {
        self.address = Some(address.to_string());
        self
    }

    /// Build the MetricsServer
    ///
    /// # Errors
    ///
    /// Returns `MetricsServerError::ConfigError` if address is not set.
    pub fn build(self) -> Result<MetricsServer, MetricsServerError> {
        let address = self
            .address
            .ok_or_else(|| MetricsServerError::ConfigError("Address is required".into()))?;

        Ok(MetricsServer::new(MetricsServerConfig { address }))
    }
}

/// Metrics server error types
#[derive(Debug, thiserror::Error)]
pub enum MetricsServerError {
    /// Configuration error (e.g., missing required field)
    #[error("Configuration error: {0}")]
    ConfigError(String),

    /// I/O error (e.g., failed to bind to address)
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    /// General server error
    #[error("Server error: {0}")]
    ServerError(String),
}

/// Prometheus metrics HTTP server
///
/// Exposes `/metrics` endpoint for Prometheus scraping and `/health` for
/// health checks.
pub struct MetricsServer {
    config: MetricsServerConfig,
    shutdown_tx: Option<oneshot::Sender<()>>,
    server_handle: Option<tokio::task::JoinHandle<()>>,
    bound_addr: Option<SocketAddr>,
}

impl std::fmt::Debug for MetricsServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MetricsServer")
            .field("config", &self.config)
            .field("is_running", &self.is_running())
            .field("bound_addr", &self.bound_addr)
            .finish()
    }
}

impl MetricsServer {
    /// Create a new metrics server with the given configuration
    pub fn new(config: MetricsServerConfig) -> Self {
        Self {
            config,
            shutdown_tx: None,
            server_handle: None,
            bound_addr: None,
        }
    }

    /// Create a builder for MetricsServer
    pub fn builder() -> MetricsServerBuilder {
        MetricsServerBuilder::default()
    }

    /// Start the metrics server
    ///
    /// Returns the actual bound address. This is useful when using port 0
    /// to get an automatically assigned port.
    ///
    /// # Errors
    ///
    /// Returns an error if the server fails to bind to the configured address.
    #[cfg_attr(feature = "tracing", tracing::instrument(
        name = "metrics_server.start",
        skip(self),
        fields(address = %self.config.address)
    ))]
    pub async fn start(&mut self) -> Result<SocketAddr, MetricsServerError> {
        let listener = TcpListener::bind(&self.config.address).await?;
        let addr = listener.local_addr()?;
        self.bound_addr = Some(addr);

        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        self.shutdown_tx = Some(shutdown_tx);

        let handle = tokio::spawn(async move {
            run_server(listener, shutdown_rx).await;
        });

        self.server_handle = Some(handle);

        #[cfg(feature = "tracing")]
        tracing::info!(%addr, "Metrics server started");

        Ok(addr)
    }

    /// Gracefully shutdown the metrics server
    #[cfg_attr(feature = "tracing", tracing::instrument(
        name = "metrics_server.shutdown",
        skip(self)
    ))]
    pub async fn shutdown(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.server_handle.take() {
            let _ = handle.await;
        }
        self.bound_addr = None;

        #[cfg(feature = "tracing")]
        tracing::info!("Metrics server stopped");
    }

    /// Check if the server is currently running
    pub fn is_running(&self) -> bool {
        self.shutdown_tx.is_some()
    }

    /// Get the bound address (only available after start)
    pub fn local_addr(&self) -> Option<SocketAddr> {
        self.bound_addr
    }
}

/// Run the HTTP server loop
async fn run_server(listener: TcpListener, mut shutdown_rx: oneshot::Receiver<()>) {
    loop {
        tokio::select! {
            biased;

            _ = &mut shutdown_rx => {
                break;
            }

            result = listener.accept() => {
                match result {
                    Ok((stream, _addr)) => {
                        let io = TokioIo::new(stream);
                        tokio::spawn(async move {
                            let _ = http1::Builder::new()
                                .serve_connection(io, service_fn(handle_request))
                                .await;
                        });
                    }
                    Err(_e) => {
                        #[cfg(feature = "tracing")]
                        tracing::warn!(error = %_e, "Failed to accept connection");
                        continue;
                    }
                }
            }
        }
    }
}

/// Handle HTTP requests
async fn handle_request(
    req: Request<hyper::body::Incoming>,
) -> Result<Response<Full<Bytes>>, Infallible> {
    let response = match (req.method(), req.uri().path()) {
        (&Method::GET, "/metrics") => metrics_handler(),
        (&Method::GET, "/health") => health_handler(),
        _ => not_found_handler(),
    };
    Ok(response)
}

/// Handle /metrics endpoint - returns Prometheus text format
fn metrics_handler() -> Response<Full<Bytes>> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();

    if encoder.encode(&metric_families, &mut buffer).is_err() {
        return build_response(
            StatusCode::INTERNAL_SERVER_ERROR,
            "text/plain",
            "Failed to encode metrics",
        );
    }

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", encoder.format_type())
        .body(Full::new(Bytes::from(buffer)))
        .unwrap()
}

/// Handle /health endpoint - returns JSON health status
fn health_handler() -> Response<Full<Bytes>> {
    build_response(StatusCode::OK, "application/json", r#"{"status":"ok"}"#)
}

/// Handle unknown endpoints - returns 404
fn not_found_handler() -> Response<Full<Bytes>> {
    build_response(StatusCode::NOT_FOUND, "text/plain", "Not Found")
}

/// Helper to build a simple response
fn build_response(status: StatusCode, content_type: &str, body: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header("Content-Type", content_type)
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_creation() {
        let config = MetricsServerConfig {
            address: "127.0.0.1:9090".to_string(),
        };
        assert_eq!(config.address, "127.0.0.1:9090");
    }

    #[test]
    fn test_builder_missing_address() {
        let result = MetricsServerBuilder::default().build();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Address is required"));
    }

    #[test]
    fn test_builder_with_address() {
        let result = MetricsServerBuilder::default()
            .address("127.0.0.1:9090")
            .build();
        assert!(result.is_ok());
    }

    #[test]
    fn test_is_running_initially_false() {
        let config = MetricsServerConfig {
            address: "127.0.0.1:0".to_string(),
        };
        let server = MetricsServer::new(config);
        assert!(!server.is_running());
    }

    #[test]
    fn test_local_addr_initially_none() {
        let config = MetricsServerConfig {
            address: "127.0.0.1:0".to_string(),
        };
        let server = MetricsServer::new(config);
        assert!(server.local_addr().is_none());
    }
}
