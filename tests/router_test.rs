//! Router Integration Tests
//!
//! RED Phase: Tests for bucket resolution and routing logic.
//! These tests define the expected behavior before implementation.

use mizuchi_uploadr::config::{
    AuthConfig, BucketConfig, Config, MetricsConfig, S3Config, ServerConfig, UploadConfig,
    ZeroCopyConfig,
};
use mizuchi_uploadr::router::{BucketResolver, RouterError, S3RequestParser};

/// Test: Route /uploads/file.txt to correct S3 bucket
#[test]
fn test_route_to_correct_bucket() {
    // Setup: Create config with multiple buckets
    let config = create_test_config();
    let resolver = BucketResolver::new(&config);

    // Test: Resolve /uploads/file.txt
    let result = resolver.resolve_bucket("/uploads/file.txt");

    // Assert: Should route to "uploads" bucket
    assert!(result.is_ok());
    let bucket_config = result.unwrap();
    assert_eq!(bucket_config.name, "uploads");
    assert_eq!(bucket_config.s3.bucket, "my-uploads-bucket");
}

/// Test: Route /documents/report.pdf to correct S3 bucket
#[test]
fn test_route_to_documents_bucket() {
    let config = create_test_config();
    let resolver = BucketResolver::new(&config);

    // Test: Resolve /documents/report.pdf
    let result = resolver.resolve_bucket("/documents/report.pdf");

    // Assert: Should route to "documents" bucket
    assert!(result.is_ok());
    let bucket_config = result.unwrap();
    assert_eq!(bucket_config.name, "documents");
    assert_eq!(bucket_config.s3.bucket, "my-documents-bucket");
}

/// Test: Reject requests to non-configured buckets
#[test]
fn test_reject_non_configured_bucket() {
    let config = create_test_config();
    let resolver = BucketResolver::new(&config);

    // Test: Try to resolve /invalid/file.txt
    let result = resolver.resolve_bucket("/invalid/file.txt");

    // Assert: Should return BucketNotFound error
    assert!(result.is_err());
    match result.unwrap_err() {
        RouterError::BucketNotFound(msg) => {
            assert!(msg.contains("invalid"));
        }
        _ => panic!("Expected BucketNotFound error"),
    }
}

/// Test: Handle multiple bucket configurations
#[test]
fn test_multiple_bucket_configurations() {
    let config = create_test_config();
    let resolver = BucketResolver::new(&config);

    // Test: Resolve multiple paths
    let uploads = resolver.resolve_bucket("/uploads/file1.txt").unwrap();
    let documents = resolver.resolve_bucket("/documents/file2.pdf").unwrap();
    let images = resolver.resolve_bucket("/images/photo.jpg").unwrap();

    // Assert: Each path routes to correct bucket
    assert_eq!(uploads.name, "uploads");
    assert_eq!(documents.name, "documents");
    assert_eq!(images.name, "images");
}

/// Test: Handle path prefix matching
#[test]
fn test_path_prefix_matching() {
    let config = create_test_config();
    let resolver = BucketResolver::new(&config);

    // Test: Nested paths should match prefix
    let result1 = resolver.resolve_bucket("/uploads/subfolder/file.txt");
    let result2 = resolver.resolve_bucket("/uploads/a/b/c/deep.txt");

    // Assert: Both should route to uploads bucket
    assert!(result1.is_ok());
    assert!(result2.is_ok());
    assert_eq!(result1.unwrap().name, "uploads");
    assert_eq!(result2.unwrap().name, "uploads");
}

/// Test: Handle root path (no bucket)
#[test]
fn test_root_path_no_bucket() {
    let config = create_test_config();
    let resolver = BucketResolver::new(&config);

    // Test: Try to resolve root path
    let result = resolver.resolve_bucket("/");

    // Assert: Should return error
    assert!(result.is_err());
}

/// Test: Handle empty path
#[test]
fn test_empty_path() {
    let config = create_test_config();
    let resolver = BucketResolver::new(&config);

    // Test: Try to resolve empty path
    let result = resolver.resolve_bucket("");

    // Assert: Should return error
    assert!(result.is_err());
}

/// Test: Extract S3 key from path
#[test]
fn test_extract_s3_key_from_path() {
    let config = create_test_config();
    let resolver = BucketResolver::new(&config);

    // Test: Resolve and extract key
    let (bucket_config, key) = resolver
        .resolve_bucket_and_key("/uploads/folder/file.txt")
        .unwrap();

    // Assert: Key should be the path after bucket prefix
    assert_eq!(bucket_config.name, "uploads");
    assert_eq!(key, "folder/file.txt");
}

/// Test: Integration with S3RequestParser
#[test]
fn test_integration_with_parser() {
    let config = create_test_config();
    let resolver = BucketResolver::new(&config);

    // Test: Parse request and resolve bucket
    let operation = S3RequestParser::parse("PUT", "/uploads/file.txt", None).unwrap();

    // Extract bucket from operation
    let bucket_name = match operation {
        mizuchi_uploadr::router::S3Operation::PutObject { bucket, .. } => bucket,
        _ => panic!("Expected PutObject operation"),
    };

    // Resolve using the extracted bucket name
    let result = resolver.resolve_bucket(&format!("/{}/file.txt", bucket_name));
    assert!(result.is_ok());
}

// Helper function to create test configuration
fn create_test_config() -> Config {
    Config {
        server: ServerConfig {
            address: "127.0.0.1:8080".to_string(),
            zero_copy: ZeroCopyConfig::default(),
        },
        buckets: vec![
            BucketConfig {
                name: "uploads".to_string(),
                path_prefix: "/uploads".to_string(),
                s3: S3Config {
                    bucket: "my-uploads-bucket".to_string(),
                    region: "us-east-1".to_string(),
                    endpoint: None,
                    access_key: None,
                    secret_key: None,
                },
                auth: AuthConfig::default(),
                upload: UploadConfig::default(),
            },
            BucketConfig {
                name: "documents".to_string(),
                path_prefix: "/documents".to_string(),
                s3: S3Config {
                    bucket: "my-documents-bucket".to_string(),
                    region: "us-east-1".to_string(),
                    endpoint: None,
                    access_key: None,
                    secret_key: None,
                },
                auth: AuthConfig::default(),
                upload: UploadConfig::default(),
            },
            BucketConfig {
                name: "images".to_string(),
                path_prefix: "/images".to_string(),
                s3: S3Config {
                    bucket: "my-images-bucket".to_string(),
                    region: "us-west-2".to_string(),
                    endpoint: None,
                    access_key: None,
                    secret_key: None,
                },
                auth: AuthConfig::default(),
                upload: UploadConfig::default(),
            },
        ],
        metrics: MetricsConfig::default(),
        tracing: None,
    }
}
