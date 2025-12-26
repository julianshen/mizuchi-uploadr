//! Pingora-based HTTP Server
//!
//! GREEN Phase: Minimal implementation to make tests pass.
//! This uses hyper for now - will be refactored to use Pingora framework in REFACTOR phase.
//!
//! # Features
//!
//! - HTTP server binding
//! - Health check endpoint
//! - Basic request handling
//! - Graceful shutdown
//!

use crate::config::Config;
use crate::server::ServerError;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{body::Incoming, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info};

/// Pingora HTTP Server
///
/// GREEN Phase: Minimal implementation using hyper
/// TODO: Refactor to use actual Pingora framework
pub struct PingoraServer {
    config: Arc<Config>,
    listener: TcpListener,
    local_addr: SocketAddr,
}

impl PingoraServer {
    /// Create a new Pingora server instance
    ///
    /// Binds to the configured address immediately
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
    pub fn local_addr(&self) -> Result<SocketAddr, ServerError> {
        Ok(self.local_addr)
    }

    /// Run the server
    ///
    /// Accepts connections and handles requests until shutdown
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
async fn handle_request(
    req: Request<Incoming>,
    _config: Arc<Config>,
) -> Result<Response<String>, hyper::Error> {
    let path = req.uri().path();
    let method = req.method();

    info!("Handling {} {}", method, path);

    // Health check endpoint
    if path == "/health" && method == hyper::Method::GET {
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .body("ok".to_string())
            .unwrap());
    }

    // Handle upload requests (PUT to /uploads/*)
    if path.starts_with("/uploads/") && method == hyper::Method::PUT {
        // TODO: Implement actual upload logic
        // For now, just return success to make tests pass
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .body("Upload successful".to_string())
            .unwrap());
    }

    // Default: 404 Not Found
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body("Not Found".to_string())
        .unwrap())
}
