//! HTTP Server for Mizuchi Uploadr
//!
//! This module provides a high-performance HTTP server for handling upload requests.
//!
//! # Architecture
//!
//! The server is built on top of `hyper` and `tokio`, providing:
//! - Async I/O for high concurrency
//! - HTTP/1.1 support
//! - Graceful shutdown
//! - Health check endpoint
//!
//! # Design Decision: Hyper vs Pingora
//!
//! While the module is named `pingora` (inspired by Cloudflare's Pingora framework),
//! the current implementation uses `hyper` directly. This decision was made because:
//!
//! 1. **Simplicity**: Hyper provides everything we need for an upload-only proxy
//! 2. **Maturity**: Hyper is battle-tested and well-documented
//! 3. **Flexibility**: Direct hyper usage gives us full control over the HTTP layer
//! 4. **Performance**: Hyper is already extremely fast for our use case
//!
//! Pingora is designed for complex proxy scenarios (load balancing, caching, etc.)
//! which are beyond the scope of Mizuchi Uploadr's upload-only design.
//!
//! # Example
//!
//! ```no_run
//! use mizuchi_uploadr::config::Config;
//! use mizuchi_uploadr::server::pingora::PingoraServer;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = Config {
//!     server: mizuchi_uploadr::config::ServerConfig {
//!         address: "127.0.0.1:0".to_string(),
//!         zero_copy: mizuchi_uploadr::config::ZeroCopyConfig::default(),
//!     },
//!     buckets: vec![],
//!     metrics: mizuchi_uploadr::config::MetricsConfig::default(),
//!     tracing: None,
//! };
//! let server = PingoraServer::new(config).await?;
//! server.run().await?;
//! # Ok(())
//! # }
//! ```
//!

use crate::config::Config;
use crate::server::ServerError;
use http_body_util::BodyExt;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{body::Incoming, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info};

/// HTTP Server for Mizuchi Uploadr
///
/// This server handles incoming HTTP requests and routes them to appropriate handlers.
///
/// # Fields
///
/// * `config` - Server configuration (shared across connections)
/// * `listener` - TCP listener for accepting connections
/// * `local_addr` - The actual address the server is bound to
pub struct PingoraServer {
    config: Arc<Config>,
    listener: TcpListener,
    local_addr: SocketAddr,
}

impl PingoraServer {
    /// Create a new HTTP server instance
    ///
    /// This method binds to the configured address immediately. If port 0 is specified,
    /// the OS will assign an available port.
    ///
    /// # Arguments
    ///
    /// * `config` - Server configuration containing the bind address
    ///
    /// # Returns
    ///
    /// * `Ok(PingoraServer)` - Successfully created and bound server
    /// * `Err(ServerError::BindError)` - Failed to parse address or bind to port
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use mizuchi_uploadr::config::Config;
    /// # use mizuchi_uploadr::server::pingora::PingoraServer;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = Config {
    ///     server: mizuchi_uploadr::config::ServerConfig {
    ///         address: "127.0.0.1:0".to_string(),
    ///         zero_copy: mizuchi_uploadr::config::ZeroCopyConfig::default(),
    ///     },
    ///     buckets: vec![],
    ///     metrics: mizuchi_uploadr::config::MetricsConfig::default(),
    ///     tracing: None,
    /// };
    /// let server = PingoraServer::new(config).await?;
    /// println!("Server bound to: {:?}", server.local_addr()?);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(config: Config) -> Result<Self, ServerError> {
        // Parse address
        let addr: SocketAddr = config
            .server
            .address
            .parse()
            .map_err(|e| ServerError::BindError(format!("Invalid address: {}", e)))?;

        // Bind to address
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| ServerError::BindError(format!("Failed to bind to {}: {}", addr, e)))?;

        // Get actual bound address (important for port 0)
        let local_addr = listener
            .local_addr()
            .map_err(|e| ServerError::BindError(format!("Failed to get local address: {}", e)))?;

        info!("Server bound to {}", local_addr);

        Ok(Self {
            config: Arc::new(config),
            listener,
            local_addr,
        })
    }

    /// Get the local address the server is bound to
    ///
    /// This is useful when binding to port 0 (OS-assigned port) to discover
    /// which port was actually assigned.
    ///
    /// # Returns
    ///
    /// The socket address (IP + port) the server is listening on
    pub fn local_addr(&self) -> Result<SocketAddr, ServerError> {
        Ok(self.local_addr)
    }

    /// Run the server
    ///
    /// Accepts incoming connections and spawns a task to handle each one.
    /// This method runs indefinitely until the server is shut down (e.g., via SIGTERM).
    ///
    /// # Behavior
    ///
    /// - Each connection is handled in a separate tokio task
    /// - Connection errors are logged but don't stop the server
    /// - The server continues accepting new connections even if some fail
    ///
    /// # Returns
    ///
    /// This method only returns if there's a fatal error accepting connections.
    /// In normal operation, it runs forever.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use mizuchi_uploadr::config::Config;
    /// # use mizuchi_uploadr::server::pingora::PingoraServer;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = Config {
    ///     server: mizuchi_uploadr::config::ServerConfig {
    ///         address: "127.0.0.1:0".to_string(),
    ///         zero_copy: mizuchi_uploadr::config::ZeroCopyConfig::default(),
    ///     },
    ///     buckets: vec![],
    ///     metrics: mizuchi_uploadr::config::MetricsConfig::default(),
    ///     tracing: None,
    /// };
    /// let server = PingoraServer::new(config).await?;
    ///
    /// // This will run forever
    /// server.run().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn run(self) -> Result<(), ServerError> {
        info!("Starting Pingora server on {}", self.local_addr);

        loop {
            // Accept connection
            let (stream, peer_addr) = match self.listener.accept().await {
                Ok(conn) => conn,
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                    continue;
                }
            };

            let config = Arc::clone(&self.config);

            // Spawn task to handle connection
            tokio::spawn(async move {
                let io = TokioIo::new(stream);

                // Create service
                let service = service_fn(move |req| {
                    let config = Arc::clone(&config);
                    async move { handle_request(req, config).await }
                });

                // Serve connection
                if let Err(e) = http1::Builder::new().serve_connection(io, service).await {
                    error!("Error serving connection from {}: {}", peer_addr, e);
                }
            });
        }
    }
}

/// Handle HTTP request
///
/// Routes incoming requests to appropriate handlers based on path and method.
///
/// # Supported Endpoints
///
/// * `GET /health` - Health check endpoint (returns "ok")
/// * `PUT /uploads/*` - Upload endpoint (placeholder for actual upload logic)
/// * All other requests return 404 Not Found
///
/// # Arguments
///
/// * `req` - The incoming HTTP request
/// * `_config` - Server configuration (currently unused, reserved for future use)
///
/// # Returns
///
/// An HTTP response with appropriate status code and body
async fn handle_request(
    req: Request<Incoming>,
    _config: Arc<Config>,
) -> Result<Response<String>, hyper::Error> {
    let path = req.uri().path().to_string();
    let method = req.method().clone();

    info!("Handling {} {}", method, path);

    // Health check endpoint
    if path == "/health" && method == hyper::Method::GET {
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/plain")
            .body("ok".to_string())
            .expect("Failed to build health check response"));
    }

    // Handle upload requests (PUT to /uploads/*)
    if path.starts_with("/uploads/") && method == hyper::Method::PUT {
        let path_clone = path.clone();
        // Consume the request body to avoid connection reset errors
        // This is important for large uploads where the body is still being transmitted
        let body = req.into_body();
        let bytes_result = body.collect().await;
        match bytes_result {
            Ok(collected) => {
                let bytes_received = collected.to_bytes().len();
                info!(
                    "Upload request to {}: {} bytes received",
                    path_clone, bytes_received
                );
            }
            Err(e) => {
                error!("Failed to read upload body: {}", e);
                return Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("Content-Type", "text/plain")
                    .body(format!("Failed to read body: {}", e))
                    .expect("Failed to build error response"));
            }
        }
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/plain")
            .body("Upload successful".to_string())
            .expect("Failed to build upload response"));
    }

    // Default: 404 Not Found
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header("Content-Type", "text/plain")
        .body("Not Found".to_string())
        .expect("Failed to build 404 response"))
}
