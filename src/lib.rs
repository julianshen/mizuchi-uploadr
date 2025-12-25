//! Mizuchi Uploadr Library
//!
//! High-performance upload-only S3 proxy with Linux zero-copy optimization.
//!
//! # Features
//!
//! - **Upload Only**: No download/list operations for security
//! - **Zero-Copy**: Uses `splice(2)`/`sendfile(2)` on Linux
//! - **S3 Compatible**: Works with AWS SDKs and tools
//! - **Flexible Auth**: JWT, SigV4, JWKS support
//! - **Fine-Grained AuthZ**: OPA and OpenFGA integration
//!
//! # Example
//!
//! ```no_run
//! use mizuchi_uploadr::{config::Config, server::Server};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = Config::load("config.yaml")?;
//!     let server = Server::new(config)?;
//!     server.run().await?;
//!     Ok(())
//! }
//! ```

pub mod auth;
pub mod authz;
pub mod config;
pub mod metrics;
pub mod router;
pub mod s3;
pub mod server;
pub mod upload;

#[cfg(feature = "tracing")]
pub mod tracing;

// Re-export commonly used types
pub use config::Config;
pub use server::Server;

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Check if zero-copy is available on this platform
#[inline]
pub fn zero_copy_available() -> bool {
    cfg!(target_os = "linux")
}
