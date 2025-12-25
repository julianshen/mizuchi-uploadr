//! OpenTelemetry tracer initialization and lifecycle management
//!
//! This module handles the initialization of the OpenTelemetry tracer provider,
//! OTLP exporter configuration, and graceful shutdown with span flushing.

use crate::config::TracingConfig;
use opentelemetry::global;
use opentelemetry_sdk::trace::{Sampler, TracerProvider};
use opentelemetry_sdk::Resource;
use std::sync::Arc;
use thiserror::Error;

/// Errors that can occur during tracing initialization
#[derive(Error, Debug)]
pub enum TracingError {
    #[error("Invalid OTLP endpoint: {0}")]
    InvalidEndpoint(String),

    #[error("Failed to initialize OTLP exporter: {0}")]
    ExporterError(String),

    #[error("Failed to initialize tracer provider: {0}")]
    ProviderError(String),

    #[error("Tracing is not enabled")]
    NotEnabled,
}

/// RAII guard for tracing lifecycle management
///
/// Automatically flushes and shuts down the tracer provider when dropped.
/// This ensures that all pending spans are exported before the application exits.
#[derive(Debug)]
pub struct TracingGuard {
    provider: Option<Arc<TracerProvider>>,
    active: bool,
}

impl TracingGuard {
    /// Create a new tracing guard with an active tracer provider
    fn new(provider: TracerProvider) -> Self {
        Self {
            provider: Some(Arc::new(provider)),
            active: true,
        }
    }

    /// Create an inactive guard (when tracing is disabled)
    fn inactive() -> Self {
        Self {
            provider: None,
            active: false,
        }
    }

    /// Check if tracing is active
    pub fn is_active(&self) -> bool {
        self.active
    }
}

impl Drop for TracingGuard {
    fn drop(&mut self) {
        if self.active {
            // Force flush all pending spans before shutdown
            if let Some(provider) = &self.provider {
                let _ = provider.force_flush();
            }
            // Shutdown the global tracer provider
            global::shutdown_tracer_provider();
        }
    }
}

/// Initialize OpenTelemetry tracing with the given configuration
///
/// Sets up the OTLP exporter, tracer provider, and batch span processor.
/// Returns a `TracingGuard` that will flush and shutdown tracing when dropped.
///
/// # Arguments
///
/// * `config` - Tracing configuration including OTLP endpoint and sampling settings
///
/// # Returns
///
/// * `Ok(TracingGuard)` - Guard that manages tracing lifecycle
/// * `Err(TracingError)` - If initialization fails
///
/// # Example
///
/// ```no_run
/// use mizuchi_uploadr::config::TracingConfig;
/// use mizuchi_uploadr::tracing::init::init_tracing;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = TracingConfig {
///     enabled: true,
///     service_name: "my-service".to_string(),
///     // ... other config
/// #   otlp: Default::default(),
/// #   sampling: Default::default(),
/// #   batch: Default::default(),
/// };
///
/// let _guard = init_tracing(&config)?;
/// // Tracing is now active
/// # Ok(())
/// # }
/// ```
pub fn init_tracing(config: &TracingConfig) -> Result<TracingGuard, TracingError> {
    // If tracing is disabled, return inactive guard
    if !config.enabled {
        return Ok(TracingGuard::inactive());
    }

    // Validate endpoint format
    let endpoint = &config.otlp.endpoint;
    if !endpoint.starts_with("http://") && !endpoint.starts_with("https://") {
        return Err(TracingError::InvalidEndpoint(format!(
            "Endpoint must start with http:// or https://, got: {}",
            endpoint
        )));
    }

    // Configure sampling strategy
    let sampler = match config.sampling.strategy.as_str() {
        "always" => Sampler::AlwaysOn,
        "never" => Sampler::AlwaysOff,
        "ratio" => Sampler::TraceIdRatioBased(config.sampling.ratio),
        "parent_based" => {
            Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(config.sampling.ratio)))
        }
        _ => Sampler::AlwaysOn, // Default to always on
    };

    // Create resource with service name
    let resource = Resource::new(vec![opentelemetry::KeyValue::new(
        "service.name",
        config.service_name.clone(),
    )]);

    // Build tracer provider with configured sampler and resource
    // TODO: Add OTLP exporter integration (requires Tokio runtime context)
    // For now, we create a basic provider that validates configuration
    // and sets up the sampling/resource correctly
    let provider = TracerProvider::builder()
        .with_config(
            opentelemetry_sdk::trace::config()
                .with_sampler(sampler)
                .with_resource(resource),
        )
        .build();

    // Set as global provider
    global::set_tracer_provider(provider.clone());

    Ok(TracingGuard::new(provider))
}

/// Explicitly shutdown tracing and flush all pending spans
///
/// This is called automatically when `TracingGuard` is dropped, but can be
/// called explicitly for more control over the shutdown process.
///
/// # Arguments
///
/// * `guard` - The tracing guard to shutdown
///
/// # Returns
///
/// * `Ok(())` - If shutdown succeeded
/// * `Err(TracingError)` - If shutdown failed
pub fn shutdown_tracing(mut guard: TracingGuard) -> Result<(), TracingError> {
    if guard.active {
        // Force flush all pending spans
        if let Some(provider) = &guard.provider {
            for result in provider.force_flush() {
                result.map_err(|e| TracingError::ProviderError(e.to_string()))?;
            }
        }
        // Mark as inactive to prevent double shutdown in Drop
        guard.active = false;
        // Shutdown global provider
        global::shutdown_tracer_provider();
    }
    Ok(())
}
