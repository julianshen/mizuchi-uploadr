//! Tests for tracing configuration
//!
//! This test suite validates the tracing configuration parsing,
//! validation, and default values following TDD methodology.

use mizuchi_uploadr::config::Config;
use serial_test::serial;

#[test]
fn test_parse_tracing_config_from_yaml() {
    // Test parsing full tracing config from YAML
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
    // Test that tracing is optional
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

    // When tracing is not specified, it should be None
    assert!(config.tracing.is_none());
}

#[test]
fn test_tracing_config_with_defaults() {
    // Test that default values are applied
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
    // Test OTLP endpoint validation
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
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Invalid OTLP endpoint"));
}

#[test]
#[serial]
fn test_environment_variable_expansion() {
    // Test environment variable expansion in config values
    // Note: This test uses #[serial] to avoid race conditions with parallel test execution

    // Preserve existing env vars to avoid clobbering
    let old_endpoint = std::env::var("OTLP_ENDPOINT").ok();
    let old_service_name = std::env::var("SERVICE_NAME").ok();

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

    // Restore original env vars to avoid cross-test interference
    match old_endpoint {
        Some(val) => std::env::set_var("OTLP_ENDPOINT", val),
        None => std::env::remove_var("OTLP_ENDPOINT"),
    }
    match old_service_name {
        Some(val) => std::env::set_var("SERVICE_NAME", val),
        None => std::env::remove_var("SERVICE_NAME"),
    }
}

#[test]
#[serial]
fn test_environment_variable_with_defaults() {
    // Test ${VAR:-default} syntax
    // Preserve and ensure the variable is NOT set
    let old_missing_var = std::env::var("MISSING_VAR").ok();
    std::env::remove_var("MISSING_VAR");

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
  service_name: "${MISSING_VAR:-default-service}"
  otlp:
    endpoint: "http://localhost:4317"
"#;

    let config: Config = serde_yaml::from_str(yaml).expect("Failed to parse YAML");

    assert!(config.tracing.is_some());
    let tracing = config.tracing.unwrap();

    // Default value should be used when env var is missing
    assert_eq!(tracing.service_name, "default-service");

    // Restore original env var
    if let Some(val) = old_missing_var {
        std::env::set_var("MISSING_VAR", val);
    }
}

#[test]
fn test_validate_sampling_ratio() {
    // Test that invalid sampling ratio is rejected
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
  sampling:
    ratio: 1.5
"#;

    let config: Config = serde_yaml::from_str(yaml).expect("Failed to parse YAML");

    let result = config.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("sampling ratio"));
}

#[test]
fn test_validate_invalid_protocol() {
    // Test that invalid protocol is rejected
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
    protocol: "invalid"
"#;

    let config: Config = serde_yaml::from_str(yaml).expect("Failed to parse YAML");

    let result = config.validate();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("protocol"));
}
