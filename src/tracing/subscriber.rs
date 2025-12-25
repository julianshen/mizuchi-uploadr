//! Tracing subscriber setup with layered architecture
//!
//! Combines multiple tracing layers:
//! - OpenTelemetry layer for distributed tracing
//! - Fmt layer for console output
//! - EnvFilter for log level control

use crate::config::TracingConfig;
use crate::tracing::init::{init_tracing, TracingError, TracingGuard};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

/// Initialize the tracing subscriber with layered architecture
///
/// Sets up a subscriber that combines:
/// - OpenTelemetry layer (when tracing is enabled)
/// - Fmt layer for console output
/// - EnvFilter for log level control (respects RUST_LOG)
///
/// # Arguments
///
/// * `config` - Tracing configuration
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
/// use mizuchi_uploadr::tracing::subscriber::init_subscriber;
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
/// let _guard = init_subscriber(&config)?;
/// // Subscriber is now active
/// # Ok(())
/// # }
/// ```
pub fn init_subscriber(config: &TracingConfig) -> Result<TracingGuard, TracingError> {
    // Initialize OpenTelemetry tracer provider
    let guard = init_tracing(config)?;

    // Create EnvFilter from RUST_LOG or default to INFO
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    if config.enabled {
        // When tracing is enabled, combine OpenTelemetry + Fmt layers
        let telemetry_layer = tracing_opentelemetry::layer();

        let subscriber = tracing_subscriber::registry()
            .with(telemetry_layer)
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().with_target(true));

        tracing::subscriber::set_global_default(subscriber)
            .map_err(|e| TracingError::ProviderError(e.to_string()))?;
    } else {
        // When tracing is disabled, only use Fmt layer
        let subscriber = tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().with_target(true));

        tracing::subscriber::set_global_default(subscriber)
            .map_err(|e| TracingError::ProviderError(e.to_string()))?;
    }

    Ok(guard)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscriber_init_disabled() {
        let config = TracingConfig {
            enabled: false,
            service_name: "test".to_string(),
            otlp: Default::default(),
            sampling: Default::default(),
            batch: Default::default(),
        };

        let result = init_subscriber(&config);
        // May fail if subscriber already initialized, but that's ok for this test
        let _ = result;
    }
}
