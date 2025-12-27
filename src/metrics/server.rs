//! Prometheus Metrics HTTP Server
//!
//! Provides an HTTP endpoint for Prometheus to scrape metrics.
//!
//! # Example
//!
//! ```no_run
//! use mizuchi_uploadr::metrics::server::{MetricsServer, MetricsServerConfig};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = MetricsServerConfig {
//!         address: "127.0.0.1:9090".to_string(),
//!     };
//!     let server = MetricsServer::new(config);
//!     let addr = server.start().await?;
//!     println!("Metrics server listening on {}", addr);
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
    /// Address to bind to (e.g., "127.0.0.1:9090")
    pub address: String,
}

/// Builder for MetricsServer
#[derive(Default)]
pub struct MetricsServerBuilder {
    address: Option<String>,
}

impl MetricsServerBuilder {
    /// Set the server address
    pub fn address(mut self, address: &str) -> Self {
        self.address = Some(address.to_string());
        self
    }

    /// Build the MetricsServer
    pub fn build(self) -> Result<MetricsServer, MetricsServerError> {
        let address = self
            .address
            .ok_or_else(|| MetricsServerError::ConfigError("Address is required".into()))?;

        Ok(MetricsServer::new(MetricsServerConfig { address }))
    }
}

/// Metrics server error
#[derive(Debug, thiserror::Error)]
pub enum MetricsServerError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Server error: {0}")]
    ServerError(String),
}

/// Prometheus metrics HTTP server
pub struct MetricsServer {
    config: MetricsServerConfig,
    shutdown_tx: Option<oneshot::Sender<()>>,
    server_handle: Option<tokio::task::JoinHandle<()>>,
}

impl MetricsServer {
    /// Create a new metrics server
    pub fn new(config: MetricsServerConfig) -> Self {
        Self {
            config,
            shutdown_tx: None,
            server_handle: None,
        }
    }

    /// Create a builder for MetricsServer
    pub fn builder() -> MetricsServerBuilder {
        MetricsServerBuilder::default()
    }

    /// Start the metrics server
    ///
    /// Returns the actual bound address (useful when using port 0)
    pub async fn start(&mut self) -> Result<SocketAddr, MetricsServerError> {
        let listener = TcpListener::bind(&self.config.address).await?;
        let addr = listener.local_addr()?;

        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        self.shutdown_tx = Some(shutdown_tx);

        let handle = tokio::spawn(async move {
            run_server(listener, shutdown_rx).await;
        });

        self.server_handle = Some(handle);

        Ok(addr)
    }

    /// Shutdown the metrics server
    pub async fn shutdown(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(handle) = self.server_handle.take() {
            let _ = handle.await;
        }
    }
}

/// Run the HTTP server loop
async fn run_server(listener: TcpListener, mut shutdown_rx: oneshot::Receiver<()>) {
    loop {
        tokio::select! {
            _ = &mut shutdown_rx => {
                break;
            }
            result = listener.accept() => {
                match result {
                    Ok((stream, _)) => {
                        let io = TokioIo::new(stream);
                        tokio::spawn(async move {
                            let _ = http1::Builder::new()
                                .serve_connection(io, service_fn(handle_request))
                                .await;
                        });
                    }
                    Err(_) => continue,
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

/// Handle /metrics endpoint
fn metrics_handler() -> Response<Full<Bytes>> {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = Vec::new();

    if encoder.encode(&metric_families, &mut buffer).is_err() {
        return Response::builder()
            .status(StatusCode::INTERNAL_SERVER_ERROR)
            .body(Full::new(Bytes::from("Failed to encode metrics")))
            .unwrap();
    }

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", encoder.format_type())
        .body(Full::new(Bytes::from(buffer)))
        .unwrap()
}

/// Handle /health endpoint
fn health_handler() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(r#"{"status":"ok"}"#)))
        .unwrap()
}

/// Handle unknown endpoints
fn not_found_handler() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Full::new(Bytes::from("Not Found")))
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
    }

    #[test]
    fn test_builder_with_address() {
        let result = MetricsServerBuilder::default()
            .address("127.0.0.1:9090")
            .build();
        assert!(result.is_ok());
    }
}
