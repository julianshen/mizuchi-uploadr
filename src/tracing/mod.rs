//! OpenTelemetry distributed tracing module
//!
//! Provides OpenTelemetry integration with OTLP export for distributed tracing.
//! Supports multiple backends including Jaeger, Tempo, and any OTLP-compatible collector.
//!
//! # Features
//!
//! - OTLP gRPC and HTTP export
//! - Configurable sampling strategies
//! - Batch span processing for performance
//! - Graceful shutdown with span flushing
//! - W3C Trace Context propagation
//!
//! # Example
//!
//! ```no_run
//! use mizuchi_uploadr::config::TracingConfig;
//! use mizuchi_uploadr::tracing::init::init_tracing;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = TracingConfig {
//!     enabled: true,
//!     service_name: "mizuchi-uploadr".to_string(),
//!     // ... other config
//! #   otlp: Default::default(),
//! #   sampling: Default::default(),
//! #   batch: Default::default(),
//! };
//!
//! let _guard = init_tracing(&config)?;
//! // Tracing is now active
//! // Guard will flush spans on drop
//! # Ok(())
//! # }
//! ```

pub mod init;
pub mod propagation;
pub mod subscriber;

pub use init::{init_tracing, shutdown_tracing, TracingGuard};
pub use subscriber::init_subscriber;
