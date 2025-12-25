//! Tests for tracing configuration
//!
//! This test suite validates the tracing configuration parsing,
//! validation, and default values following TDD methodology.

use mizuchi_uploadr::config::{Config, TracingConfig};
use std::collections::HashMap;

#[test]
fn test_parse_tracing_config_from_yaml() {
    // RED: This test will fail because TracingConfig doesn't exist yet
    let yaml = r#"
server:
  address: "0.0.0.0:8080"

buckets:
  - name: "test-bucket"
    path_prefix: "/uploads"
    s3:
      bucket: "my-s3-bucket"
      region: "us-east-1"

tracing:
  enabled: true
  service_name: "mizuchi-uploadr"
  otlp:
    endpoint: "http://localhost:4317"
    protocol: "grpc"
    timeout_seconds: 10
    compression: "gzip"
  sampling:
    strategy: "parent_based"
    ratio: 0.1
  batch:
    max_queue_size: 2048
    scheduled_delay_millis: 5000
    max_export_batch_size: 512
"#;

    let config: Config = serde_yaml::from_str(yaml).expect("Failed to parse YAML");
    
    assert!(config.tracing.is_some());
    let tracing = config.tracing.unwrap();
    
    assert!(tracing.enabled);
    assert_eq!(tracing.service_name, "mizuchi-uploadr");
    
    // OTLP config
    assert_eq!(tracing.otlp.endpoint, "http://localhost:4317");
    assert_eq!(tracing.otlp.protocol, "grpc");
    assert_eq!(tracing.otlp.timeout_seconds, 10);
    assert_eq!(tracing.otlp.compression, Some("gzip".to_string()));
    
    // Sampling config
    assert_eq!(tracing.sampling.strategy, "parent_based");
    assert_eq!(tracing.sampling.ratio, 0.1);
    
    // Batch config
    assert_eq!(tracing.batch.max_queue_size, 2048);
    assert_eq!(tracing.batch.scheduled_delay_millis, 5000);
    assert_eq!(tracing.batch.max_export_batch_size, 512);
}

#[test]
fn test_tracing_config_defaults_when_disabled() {
    // RED: This test will fail because TracingConfig doesn't exist yet
    let yaml = r#"
server:
  address: "0.0.0.0:8080"

buckets:
  - name: "test-bucket"
    path_prefix: "/uploads"
    s3:
      bucket: "my-s3-bucket"
      region: "us-east-1"
"#;

    let config: Config = serde_yaml::from_str(yaml).expect("Failed to parse YAML");
    
    // When tracing is not specified, it should be None or have default disabled
    assert!(config.tracing.is_none() || !config.tracing.unwrap().enabled);
}

#[test]
fn test_tracing_config_with_defaults() {
    // RED: This test will fail because TracingConfig doesn't exist yet
    let yaml = r#"
server:
  address: "0.0.0.0:8080"

buckets:
  - name: "test-bucket"
    path_prefix: "/uploads"
    s3:
      bucket: "my-s3-bucket"
      region: "us-east-1"

tracing:
  enabled: true
  service_name: "mizuchi-uploadr"
  otlp:
    endpoint: "http://localhost:4317"
"#;

    let config: Config = serde_yaml::from_str(yaml).expect("Failed to parse YAML");
    
    assert!(config.tracing.is_some());
    let tracing = config.tracing.unwrap();
    
    // Check defaults are applied
    assert_eq!(tracing.otlp.protocol, "grpc"); // default
    assert_eq!(tracing.otlp.timeout_seconds, 10); // default
    assert_eq!(tracing.sampling.strategy, "always"); // default
    assert_eq!(tracing.sampling.ratio, 1.0); // default
}

#[test]
fn test_validate_otlp_endpoint_url() {
    // RED: This test will fail because TracingConfig doesn't exist yet
    let yaml = r#"
server:
  address: "0.0.0.0:8080"

buckets:
  - name: "test-bucket"
    path_prefix: "/uploads"
    s3:
      bucket: "my-s3-bucket"
      region: "us-east-1"

tracing:
  enabled: true
  service_name: "mizuchi-uploadr"
  otlp:
    endpoint: "invalid-url"
"#;

    let config: Config = serde_yaml::from_str(yaml).expect("Failed to parse YAML");
    
    // Validation should fail for invalid URL
    let result = config.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid OTLP endpoint"));
}

#[test]
fn test_environment_variable_expansion() {
    // RED: This test will fail because environment variable expansion doesn't exist yet
    std::env::set_var("OTLP_ENDPOINT", "http://jaeger:4317");
    std::env::set_var("SERVICE_NAME", "test-service");
    
    let yaml = r#"
server:
  address: "0.0.0.0:8080"

buckets:
  - name: "test-bucket"
    path_prefix: "/uploads"
    s3:
      bucket: "my-s3-bucket"
      region: "us-east-1"

tracing:
  enabled: true
  service_name: "${SERVICE_NAME}"
  otlp:
    endpoint: "${OTLP_ENDPOINT}"
"#;

    let config: Config = serde_yaml::from_str(yaml).expect("Failed to parse YAML");
    
    assert!(config.tracing.is_some());
    let tracing = config.tracing.unwrap();
    
    // Environment variables should be expanded
    assert_eq!(tracing.service_name, "test-service");
    assert_eq!(tracing.otlp.endpoint, "http://jaeger:4317");
    
    // Cleanup
    std::env::remove_var("OTLP_ENDPOINT");
    std::env::remove_var("SERVICE_NAME");
}

