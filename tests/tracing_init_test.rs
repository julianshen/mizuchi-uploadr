//! Tests for OpenTelemetry tracing initialization
//!
//! This test suite validates the tracing initialization logic,
//! OTLP exporter setup, and graceful shutdown following TDD methodology.
//!
//! Phase 2.1: Tracing Initialization Module (RED phase)

use mizuchi_uploadr::config::{BatchConfig, OtlpConfig, SamplingConfig, TracingConfig};
use mizuchi_uploadr::tracing::init::{init_tracing, shutdown_tracing};

#[test]
fn test_init_tracing_with_valid_config() {
    // Test: Initialize tracer with valid configuration
    let config = TracingConfig {
        enabled: true,
        service_name: "test-service".to_string(),
        otlp: OtlpConfig {
            endpoint: "http://localhost:4317".to_string(),
            protocol: "grpc".to_string(),
            timeout_seconds: 10,
            compression: Some("gzip".to_string()),
        },
        sampling: SamplingConfig {
            strategy: "always".to_string(),
            ratio: 1.0,
        },
        batch: BatchConfig {
            max_queue_size: 2048,
            scheduled_delay_millis: 5000,
            max_export_batch_size: 512,
        },
    };

    let result = init_tracing(&config);

    // Should successfully initialize and return a guard
    assert!(result.is_ok());
    let guard = result.unwrap();

    // Guard should be valid
    assert!(guard.is_active());
}

#[test]
fn test_init_tracing_when_disabled() {
    // Test: When tracing is disabled, initialization should be a no-op
    let config = TracingConfig {
        enabled: false,
        service_name: "test-service".to_string(),
        otlp: OtlpConfig {
            endpoint: "http://localhost:4317".to_string(),
            protocol: "grpc".to_string(),
            timeout_seconds: 10,
            compression: None,
        },
        sampling: SamplingConfig {
            strategy: "never".to_string(),
            ratio: 0.0,
        },
        batch: BatchConfig::default(),
    };

    let result = init_tracing(&config);

    // Should succeed but not actually initialize tracing
    assert!(result.is_ok());
    let guard = result.unwrap();

    // Guard should indicate tracing is not active
    assert!(!guard.is_active());
}

#[test]
fn test_init_tracing_with_invalid_endpoint() {
    // Test: Invalid OTLP endpoint should return error
    let config = TracingConfig {
        enabled: true,
        service_name: "test-service".to_string(),
        otlp: OtlpConfig {
            endpoint: "invalid://endpoint".to_string(),
            protocol: "grpc".to_string(),
            timeout_seconds: 10,
            compression: None,
        },
        sampling: SamplingConfig {
            strategy: "always".to_string(),
            ratio: 1.0,
        },
        batch: BatchConfig::default(),
    };

    let result = init_tracing(&config);

    // Should fail with appropriate error
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("endpoint"));
}

#[test]
fn test_graceful_shutdown() {
    // Test: Graceful shutdown flushes pending spans
    let config = TracingConfig {
        enabled: true,
        service_name: "test-service".to_string(),
        otlp: OtlpConfig {
            endpoint: "http://localhost:4317".to_string(),
            protocol: "grpc".to_string(),
            timeout_seconds: 10,
            compression: None,
        },
        sampling: SamplingConfig {
            strategy: "always".to_string(),
            ratio: 1.0,
        },
        batch: BatchConfig::default(),
    };

    let guard = init_tracing(&config).expect("Failed to initialize tracing");

    // Shutdown should succeed
    let result = shutdown_tracing(guard);
    assert!(result.is_ok());
}

#[test]
fn test_tracing_guard_drop() {
    // Test: Dropping the guard should trigger shutdown
    let config = TracingConfig {
        enabled: true,
        service_name: "test-service".to_string(),
        otlp: OtlpConfig {
            endpoint: "http://localhost:4317".to_string(),
            protocol: "grpc".to_string(),
            timeout_seconds: 10,
            compression: None,
        },
        sampling: SamplingConfig {
            strategy: "always".to_string(),
            ratio: 1.0,
        },
        batch: BatchConfig::default(),
    };

    {
        let _guard = init_tracing(&config).expect("Failed to initialize tracing");
        // Guard should be active within this scope
    } // Guard dropped here - should trigger shutdown

    // If we reach here without panic, the drop worked correctly
}
