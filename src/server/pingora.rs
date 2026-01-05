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

use crate::auth::jwt::JwtAuthenticator;
use crate::auth::{AuthError, AuthRequest, Authenticator};
use crate::config::{BucketConfig, Config};
use crate::s3::{S3Client, S3ClientConfig};
use crate::server::ServerError;
use http_body_util::BodyExt;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{body::Incoming, Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info, warn};

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

/// Find a bucket configuration that matches the request path
///
/// This function matches on path prefix boundaries and returns the longest matching prefix
/// to avoid mis-routing (e.g., `/uploads2/...` should not match `/uploads`).
fn find_bucket_for_path<'a>(config: &'a Config, path: &str) -> Option<&'a BucketConfig> {
    config
        .buckets
        .iter()
        .filter(|bucket| {
            // Match on prefix boundary: path must either equal prefix exactly,
            // or continue with '/' after prefix
            if path == bucket.path_prefix {
                true
            } else if path.starts_with(&bucket.path_prefix) {
                // Check that the character after prefix is '/' to ensure boundary match
                let after_prefix = &path[bucket.path_prefix.len()..];
                after_prefix.starts_with('/')
            } else {
                false
            }
        })
        // Return the longest matching prefix to handle overlapping prefixes correctly
        .max_by_key(|bucket| bucket.path_prefix.len())
}

/// Build AuthRequest from hyper Request headers
fn build_auth_request(req: &Request<Incoming>) -> AuthRequest {
    let mut headers = HashMap::new();
    for (name, value) in req.headers() {
        if let Ok(v) = value.to_str() {
            headers.insert(name.to_string().to_lowercase(), v.to_string());
        }
    }

    AuthRequest {
        headers,
        query: req.uri().query().map(|q| q.to_string()),
        method: req.method().to_string(),
        path: req.uri().path().to_string(),
    }
}

/// Handle HTTP request
///
/// Routes incoming requests to appropriate handlers based on path and method.
///
/// # Supported Endpoints
///
/// * `GET /health` - Health check endpoint (returns "ok")
/// * `PUT /{path_prefix}/*` - Upload endpoint (forwards to S3 backend)
/// * All other requests return 404 Not Found
///
/// # Authentication
///
/// If a bucket has `auth.enabled = true` and JWT config, the request must include
/// a valid JWT token in the `Authorization: Bearer <token>` header.
///
/// # Arguments
///
/// * `req` - The incoming HTTP request
/// * `config` - Server configuration with bucket definitions
///
/// # Returns
///
/// An HTTP response with appropriate status code and body
async fn handle_request(
    req: Request<Incoming>,
    config: Arc<Config>,
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

    // Find matching bucket for the path
    let bucket = match find_bucket_for_path(&config, &path) {
        Some(b) => b,
        None => {
            info!("No bucket configured for path: {}", path);
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .header("Content-Type", "text/plain")
                .body("Not Found".to_string())
                .expect("Failed to build 404 response"));
        }
    };

    // Handle upload requests (PUT)
    if method == hyper::Method::PUT {
        // Authenticate if auth is enabled for this bucket
        if bucket.auth.enabled {
            if let Some(ref jwt_config) = bucket.auth.jwt {
                // Create JWT authenticator from config
                let secret = match &jwt_config.secret {
                    Some(s) => s,
                    None => {
                        error!(
                            "JWT auth enabled but no secret configured for bucket {}",
                            bucket.name
                        );
                        return Ok(Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .header("Content-Type", "text/plain")
                            .body("Server configuration error".to_string())
                            .expect("Failed to build error response"));
                    }
                };

                // Create JWT authenticator based on configured algorithm
                let authenticator = match jwt_config.algorithm.to_uppercase().as_str() {
                    "HS256" => JwtAuthenticator::new_hs256(secret),
                    "RS256" => match JwtAuthenticator::new_rs256(secret) {
                        Ok(auth) => auth,
                        Err(e) => {
                            error!("Failed to create RS256 authenticator: {}", e);
                            return Ok(Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .header("Content-Type", "text/plain")
                                .body("Server configuration error".to_string())
                                .expect("Failed to build error response"));
                        }
                    },
                    "ES256" => match JwtAuthenticator::new_es256(secret) {
                        Ok(auth) => auth,
                        Err(e) => {
                            error!("Failed to create ES256 authenticator: {}", e);
                            return Ok(Response::builder()
                                .status(StatusCode::INTERNAL_SERVER_ERROR)
                                .header("Content-Type", "text/plain")
                                .body("Server configuration error".to_string())
                                .expect("Failed to build error response"));
                        }
                    },
                    alg => {
                        error!("Unsupported JWT algorithm configured: {}", alg);
                        return Ok(Response::builder()
                            .status(StatusCode::INTERNAL_SERVER_ERROR)
                            .header("Content-Type", "text/plain")
                            .body("Server configuration error".to_string())
                            .expect("Failed to build error response"));
                    }
                };
                let auth_request = build_auth_request(&req);

                match authenticator.authenticate(&auth_request).await {
                    Ok(result) => {
                        info!("Authenticated user: {}", result.subject);
                    }
                    Err(AuthError::MissingAuth) => {
                        warn!("Missing authentication for {}", path);
                        return Ok(Response::builder()
                            .status(StatusCode::UNAUTHORIZED)
                            .header("Content-Type", "text/plain")
                            .header("WWW-Authenticate", "Bearer")
                            .body("Missing authentication".to_string())
                            .expect("Failed to build 401 response"));
                    }
                    Err(AuthError::TokenExpired) => {
                        warn!("Expired token for {}", path);
                        return Ok(Response::builder()
                            .status(StatusCode::UNAUTHORIZED)
                            .header("Content-Type", "text/plain")
                            .header("WWW-Authenticate", "Bearer error=\"invalid_token\", error_description=\"Token expired\"")
                            .body("Token expired".to_string())
                            .expect("Failed to build 401 response"));
                    }
                    Err(AuthError::InvalidSignature) | Err(AuthError::InvalidToken(_)) => {
                        warn!("Invalid token for {}", path);
                        return Ok(Response::builder()
                            .status(StatusCode::UNAUTHORIZED)
                            .header("Content-Type", "text/plain")
                            .header("WWW-Authenticate", "Bearer error=\"invalid_token\"")
                            .body("Invalid token".to_string())
                            .expect("Failed to build 401 response"));
                    }
                    Err(e) => {
                        error!("Authentication error: {}", e);
                        return Ok(Response::builder()
                            .status(StatusCode::UNAUTHORIZED)
                            .header("Content-Type", "text/plain")
                            .body(format!("Authentication failed: {}", e))
                            .expect("Failed to build 401 response"));
                    }
                }
            } else {
                // Fail-closed: auth enabled but no JWT config means deny access
                error!("Auth enabled but no JWT config for bucket {}", bucket.name);
                return Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header("Content-Type", "text/plain")
                    .body("Server configuration error".to_string())
                    .expect("Failed to build error response"));
            }
        }

        // Extract content type from request
        let content_type = req
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        // Collect the request body
        let body = req.into_body();
        let bytes_result = body.collect().await;
        let body_bytes = match bytes_result {
            Ok(collected) => collected.to_bytes(),
            Err(e) => {
                error!("Failed to read upload body: {}", e);
                return Ok(Response::builder()
                    .status(StatusCode::BAD_REQUEST)
                    .header("Content-Type", "text/plain")
                    .body(format!("Failed to read body: {}", e))
                    .expect("Failed to build error response"));
            }
        };

        info!(
            "Upload request to {}: {} bytes received",
            path,
            body_bytes.len()
        );

        // Create S3 client and upload
        let s3_config = S3ClientConfig {
            bucket: bucket.s3.bucket.clone(),
            region: bucket.s3.region.clone(),
            endpoint: bucket.s3.endpoint.clone(),
            access_key: bucket.s3.access_key.clone(),
            secret_key: bucket.s3.secret_key.clone(),
            retry: None,
            timeout: None,
        };

        let s3_client = match S3Client::new(s3_config) {
            Ok(client) => client,
            Err(e) => {
                error!("Failed to create S3 client: {}", e);
                return Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header("Content-Type", "text/plain")
                    .body("Failed to create S3 client".to_string())
                    .expect("Failed to build error response"));
            }
        };

        // Extract the S3 key from the path (remove the path prefix)
        let s3_key = path
            .strip_prefix(&bucket.path_prefix)
            .unwrap_or(&path)
            .trim_start_matches('/');

        // Validate S3 key is not empty
        if s3_key.is_empty() {
            warn!("Empty S3 key for path: {}", path);
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("Content-Type", "text/plain")
                .body("Invalid key: object key cannot be empty".to_string())
                .expect("Failed to build error response"));
        }

        // Upload to S3
        match s3_client
            .put_object(
                s3_key,
                body_bytes,
                content_type.as_deref(),
            )
            .await
        {
            Ok(response) => {
                info!("Upload successful, ETag: {}", response.etag);
                return Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header("Content-Type", "text/plain")
                    .header("ETag", &response.etag)
                    .body("Upload successful".to_string())
                    .expect("Failed to build upload response"));
            }
            Err(e) => {
                error!("S3 upload failed: {}", e);
                return Ok(Response::builder()
                    .status(StatusCode::INTERNAL_SERVER_ERROR)
                    .header("Content-Type", "text/plain")
                    .body(format!("Upload failed: {}", e))
                    .expect("Failed to build error response"));
            }
        }
    }

    // Default: 404 Not Found
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header("Content-Type", "text/plain")
        .body("Not Found".to_string())
        .expect("Failed to build 404 response"))
}
