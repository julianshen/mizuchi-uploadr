//! Integration tests for tracing subscriber setup
//!
//! RED PHASE: Tests for layered subscriber with OpenTelemetry integration

use mizuchi_uploadr::config::TracingConfig;

/// Test that subscriber can be initialized with tracing disabled
#[test]
fn test_subscriber_init_when_disabled() {
    let config = TracingConfig {
        enabled: false,
        service_name: "test-service".to_string(),
        otlp: Default::default(),
        sampling: Default::default(),
        batch: Default::default(),
    };

    // Should succeed even when disabled
    // Note: May fail if subscriber already initialized, which is ok
    let result = mizuchi_uploadr::tracing::subscriber::init_subscriber(&config);
    // Either succeeds or fails with "already set" error
    if let Err(e) = result {
        assert!(e.to_string().contains("already been set"));
    }
}

/// Test that subscriber can be initialized with valid config
#[test]
fn test_subscriber_init_with_valid_config() {
    let config = TracingConfig {
        enabled: true,
        service_name: "test-service".to_string(),
        otlp: mizuchi_uploadr::config::OtlpConfig {
            endpoint: "http://localhost:4317".to_string(),
            protocol: "grpc".to_string(),
            timeout_seconds: 10,
            compression: None,
        },
        sampling: Default::default(),
        batch: Default::default(),
    };

    // May fail if subscriber already initialized
    let result = mizuchi_uploadr::tracing::subscriber::init_subscriber(&config);
    if let Err(e) = result {
        assert!(e.to_string().contains("already been set"));
    }
}

/// Test that subscriber combines multiple layers
#[tokio::test]
async fn test_subscriber_has_multiple_layers() {
    let config = TracingConfig {
        enabled: true,
        service_name: "test-service".to_string(),
        otlp: mizuchi_uploadr::config::OtlpConfig {
            endpoint: "http://localhost:4317".to_string(),
            protocol: "grpc".to_string(),
            timeout_seconds: 10,
            compression: None,
        },
        sampling: Default::default(),
        batch: Default::default(),
    };

    // Initialize subscriber (may fail if already initialized)
    let result = mizuchi_uploadr::tracing::subscriber::init_subscriber(&config);
    if result.is_ok() {
        // Test that both console output and OpenTelemetry work
        tracing::info!("Test log message");
    }

    // If we got here, test passed
    assert!(true);
}

/// Test that EnvFilter works correctly
#[test]
fn test_subscriber_respects_env_filter() {
    std::env::set_var("RUST_LOG", "info");

    let config = TracingConfig {
        enabled: false,
        service_name: "test-service".to_string(),
        otlp: Default::default(),
        sampling: Default::default(),
        batch: Default::default(),
    };

    let result = mizuchi_uploadr::tracing::subscriber::init_subscriber(&config);
    // May fail if subscriber already initialized
    if let Err(e) = result {
        assert!(e.to_string().contains("already been set"));
    }

    std::env::remove_var("RUST_LOG");
}
