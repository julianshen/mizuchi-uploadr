//! S3 Client Pool Integration Tests
//!
//! Tests for S3 client pool with credential loading and connection pooling.
//! These tests follow TDD methodology - RED phase: tests are written before implementation.
//!
//! # Task 8: Phase 1.3 - S3 Client Integration
//!
//! ## Test Coverage
//!
//! - S3ClientPool creation and initialization
//! - Credential loading from environment variables
//! - Credential loading from configuration
//! - Connection pooling (client reuse)
//! - SigV4 request signing verification

#[cfg(test)]
mod tests {
    // Import the types we'll be creating
    // These imports will fail until we implement the types (RED phase)

    // ========================================================================
    // TEST: S3ClientPool Creation
    // ========================================================================

    /// Test that S3ClientPool can be created with AWS SDK configuration
    ///
    /// This test verifies that:
    /// - S3ClientPool struct exists
    /// - It can be initialized from a Config
    /// - Multiple bucket configs create pooled clients
    #[tokio::test]
    async fn test_s3_client_pool_creation() {
        use mizuchi_uploadr::config::{
            AuthConfig, BucketConfig, Config, MetricsConfig, S3Config, ServerConfig,
            UploadConfig, ZeroCopyConfig,
        };
        use mizuchi_uploadr::s3::S3ClientPool; // This should fail - type doesn't exist yet

        let config = Config {
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
                        access_key: Some("AKIAIOSFODNN7EXAMPLE".to_string()),
                        secret_key: Some("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string()),
                    },
                    auth: AuthConfig::default(),
                    upload: UploadConfig::default(),
                },
                BucketConfig {
                    name: "attachments".to_string(),
                    path_prefix: "/attachments".to_string(),
                    s3: S3Config {
                        bucket: "my-attachments-bucket".to_string(),
                        region: "us-west-2".to_string(),
                        endpoint: None,
                        access_key: Some("AKIAIOSFODNN7EXAMPLE".to_string()),
                        secret_key: Some("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string()),
                    },
                    auth: AuthConfig::default(),
                    upload: UploadConfig::default(),
                },
            ],
            metrics: MetricsConfig::default(),
            tracing: None,
        };

        // Create the pool - should succeed
        let pool = S3ClientPool::new(&config).await.unwrap();

        // Pool should have 2 clients (one per bucket)
        assert_eq!(pool.client_count(), 2);
    }

    /// Test that S3ClientPool can retrieve a client for a specific bucket
    #[tokio::test]
    async fn test_s3_client_pool_get_client() {
        use mizuchi_uploadr::config::{
            AuthConfig, BucketConfig, Config, MetricsConfig, S3Config, ServerConfig,
            UploadConfig, ZeroCopyConfig,
        };
        use mizuchi_uploadr::s3::S3ClientPool;

        let config = Config {
            server: ServerConfig {
                address: "127.0.0.1:8080".to_string(),
                zero_copy: ZeroCopyConfig::default(),
            },
            buckets: vec![BucketConfig {
                name: "uploads".to_string(),
                path_prefix: "/uploads".to_string(),
                s3: S3Config {
                    bucket: "my-bucket".to_string(),
                    region: "us-east-1".to_string(),
                    endpoint: None,
                    access_key: Some("AKIAIOSFODNN7EXAMPLE".to_string()),
                    secret_key: Some("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string()),
                },
                auth: AuthConfig::default(),
                upload: UploadConfig::default(),
            }],
            metrics: MetricsConfig::default(),
            tracing: None,
        };

        let pool = S3ClientPool::new(&config).await.unwrap();

        // Should be able to get a client for the configured bucket
        let client = pool.get_client("uploads");
        assert!(client.is_some());

        // Should return None for unconfigured bucket
        let missing = pool.get_client("nonexistent");
        assert!(missing.is_none());
    }

    // ========================================================================
    // TEST: Credential Loading from Environment
    // ========================================================================

    /// Test that credentials can be loaded from environment variables
    ///
    /// AWS SDK standard environment variables:
    /// - AWS_ACCESS_KEY_ID
    /// - AWS_SECRET_ACCESS_KEY
    /// - AWS_REGION (optional)
    #[tokio::test]
    #[serial_test::serial]
    async fn test_credentials_from_environment() {
        use mizuchi_uploadr::s3::CredentialsProvider;

        // Set environment variables
        std::env::set_var("AWS_ACCESS_KEY_ID", "test-access-key");
        std::env::set_var("AWS_SECRET_ACCESS_KEY", "test-secret-key");

        let provider = CredentialsProvider::from_env().await;
        assert!(provider.is_ok());

        let creds = provider.unwrap();
        assert_eq!(creds.access_key_id(), "test-access-key");
        assert_eq!(creds.secret_access_key(), "test-secret-key");

        // Cleanup
        std::env::remove_var("AWS_ACCESS_KEY_ID");
        std::env::remove_var("AWS_SECRET_ACCESS_KEY");
    }

    /// Test that credentials can be loaded from S3Config
    #[tokio::test]
    async fn test_credentials_from_config() {
        use mizuchi_uploadr::config::S3Config;
        use mizuchi_uploadr::s3::CredentialsProvider;

        let s3_config = S3Config {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            endpoint: None,
            access_key: Some("config-access-key".to_string()),
            secret_key: Some("config-secret-key".to_string()),
        };

        let provider = CredentialsProvider::from_config(&s3_config);
        assert!(provider.is_ok());

        let creds = provider.unwrap();
        assert_eq!(creds.access_key_id(), "config-access-key");
        assert_eq!(creds.secret_access_key(), "config-secret-key");
    }

    /// Test that credentials fail gracefully when not provided
    #[tokio::test]
    async fn test_credentials_missing() {
        use mizuchi_uploadr::config::S3Config;
        use mizuchi_uploadr::s3::CredentialsProvider;

        let s3_config = S3Config {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            endpoint: None,
            access_key: None, // No credentials
            secret_key: None,
        };

        let provider = CredentialsProvider::from_config(&s3_config);
        // Should fail or return an error when credentials are missing
        assert!(provider.is_err());
    }

    // ========================================================================
    // TEST: Connection Pooling
    // ========================================================================

    /// Test that multiple requests reuse the same client (connection pooling)
    ///
    /// Verifies that the pool returns the same logical client for the same bucket.
    /// Note: We test behavioral equality (same bucket/region) rather than pointer
    /// equality since the pool returns references from Arc containers.
    #[tokio::test]
    async fn test_connection_pooling_reuses_client() {
        use mizuchi_uploadr::config::{
            AuthConfig, BucketConfig, Config, MetricsConfig, S3Config, ServerConfig,
            UploadConfig, ZeroCopyConfig,
        };
        use mizuchi_uploadr::s3::S3ClientPool;

        let config = Config {
            server: ServerConfig {
                address: "127.0.0.1:8080".to_string(),
                zero_copy: ZeroCopyConfig::default(),
            },
            buckets: vec![BucketConfig {
                name: "uploads".to_string(),
                path_prefix: "/uploads".to_string(),
                s3: S3Config {
                    bucket: "my-bucket".to_string(),
                    region: "us-east-1".to_string(),
                    endpoint: Some("http://localhost:9000".to_string()),
                    access_key: Some("AKIAIOSFODNN7EXAMPLE".to_string()),
                    secret_key: Some("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string()),
                },
                auth: AuthConfig::default(),
                upload: UploadConfig::default(),
            }],
            metrics: MetricsConfig::default(),
            tracing: None,
        };

        let pool = S3ClientPool::new(&config).await.unwrap();

        // Get client multiple times - should return same logical client
        let client1 = pool.get_client("uploads");
        let client2 = pool.get_client("uploads");

        // Both should be Some
        assert!(client1.is_some());
        assert!(client2.is_some());

        // Verify they represent the same logical client by checking configuration
        let c1 = client1.unwrap();
        let c2 = client2.unwrap();
        assert_eq!(c1.bucket(), c2.bucket(), "Clients should have same bucket");
        assert_eq!(c1.region(), c2.region(), "Clients should have same region");
        assert_eq!(c1.endpoint(), c2.endpoint(), "Clients should have same endpoint");
    }

    // ========================================================================
    // TEST: SigV4 Request Signing
    // ========================================================================

    /// Test that requests include proper SigV4 Authorization header
    #[tokio::test]
    async fn test_sigv4_request_signing() {
        use bytes::Bytes;
        use mizuchi_uploadr::config::{
            AuthConfig, BucketConfig, Config, MetricsConfig, S3Config, ServerConfig,
            UploadConfig, ZeroCopyConfig,
        };
        use mizuchi_uploadr::s3::S3ClientPool;
        use wiremock::matchers::{header_exists, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // Expect requests to have Authorization header (SigV4 signature)
        Mock::given(method("PUT"))
            .and(path("/test-key"))
            .and(header_exists("Authorization")) // SigV4 requires Authorization header
            .and(header_exists("x-amz-date")) // SigV4 requires x-amz-date header
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("ETag", "\"abc123\""),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let config = Config {
            server: ServerConfig {
                address: "127.0.0.1:8080".to_string(),
                zero_copy: ZeroCopyConfig::default(),
            },
            buckets: vec![BucketConfig {
                name: "uploads".to_string(),
                path_prefix: "/uploads".to_string(),
                s3: S3Config {
                    bucket: "test-bucket".to_string(),
                    region: "us-east-1".to_string(),
                    endpoint: Some(mock_server.uri()),
                    access_key: Some("AKIAIOSFODNN7EXAMPLE".to_string()),
                    secret_key: Some("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string()),
                },
                auth: AuthConfig::default(),
                upload: UploadConfig::default(),
            }],
            metrics: MetricsConfig::default(),
            tracing: None,
        };

        let pool = S3ClientPool::new(&config).await.unwrap();
        let client = pool.get_client("uploads").unwrap();

        // Make a request - should include SigV4 headers
        let body = Bytes::from("test data");
        let result = client.put_object("test-key", body, Some("text/plain")).await;

        // Request should succeed (mock server validates headers)
        assert!(result.is_ok(), "Request failed: {:?}", result.err());
    }

    /// Test that SigV4 signing works with large request body
    ///
    /// This test verifies that SigV4 signing works correctly with larger payloads.
    #[tokio::test]
    async fn test_sigv4_with_large_body() {
        use bytes::Bytes;
        use mizuchi_uploadr::config::{
            AuthConfig, BucketConfig, Config, MetricsConfig, S3Config, ServerConfig,
            UploadConfig, ZeroCopyConfig,
        };
        use mizuchi_uploadr::s3::S3ClientPool;
        use wiremock::matchers::{header_exists, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // Verify SigV4 works with larger body (10KB)
        Mock::given(method("PUT"))
            .and(path("/large-test-key"))
            .and(header_exists("Authorization"))
            .and(header_exists("x-amz-date"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("ETag", "\"large-etag\""),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let config = Config {
            server: ServerConfig {
                address: "127.0.0.1:8080".to_string(),
                zero_copy: ZeroCopyConfig::default(),
            },
            buckets: vec![BucketConfig {
                name: "uploads".to_string(),
                path_prefix: "/uploads".to_string(),
                s3: S3Config {
                    bucket: "test-bucket".to_string(),
                    region: "us-east-1".to_string(),
                    endpoint: Some(mock_server.uri()),
                    access_key: Some("AKIAIOSFODNN7EXAMPLE".to_string()),
                    secret_key: Some("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string()),
                },
                auth: AuthConfig::default(),
                upload: UploadConfig::default(),
            }],
            metrics: MetricsConfig::default(),
            tracing: None,
        };

        let pool = S3ClientPool::new(&config).await.unwrap();
        let client = pool.get_client("uploads").unwrap();

        // Create a 10KB body
        let body = Bytes::from(vec![b'x'; 10 * 1024]);
        let result = client.put_object("large-test-key", body, Some("application/octet-stream")).await;

        assert!(result.is_ok(), "Large body signed request failed: {:?}", result.err());
    }

    // ========================================================================
    // TEST: Error Handling
    // ========================================================================

    /// Test that pool creation fails gracefully with invalid config
    #[tokio::test]
    async fn test_pool_creation_with_empty_buckets() {
        use mizuchi_uploadr::config::{Config, MetricsConfig, ServerConfig, ZeroCopyConfig};
        use mizuchi_uploadr::s3::S3ClientPool;

        let config = Config {
            server: ServerConfig {
                address: "127.0.0.1:8080".to_string(),
                zero_copy: ZeroCopyConfig::default(),
            },
            buckets: vec![], // No buckets
            metrics: MetricsConfig::default(),
            tracing: None,
        };

        // Pool creation should succeed but with 0 clients
        let pool = S3ClientPool::new(&config).await.unwrap();
        assert_eq!(pool.client_count(), 0);
    }

    // ========================================================================
    // TEST: Pool Client Configuration
    // ========================================================================

    /// Test that pool clients inherit correct regional endpoint
    #[tokio::test]
    async fn test_pool_client_regional_endpoint() {
        use mizuchi_uploadr::config::{
            AuthConfig, BucketConfig, Config, MetricsConfig, S3Config, ServerConfig,
            UploadConfig, ZeroCopyConfig,
        };
        use mizuchi_uploadr::s3::S3ClientPool;

        let config = Config {
            server: ServerConfig {
                address: "127.0.0.1:8080".to_string(),
                zero_copy: ZeroCopyConfig::default(),
            },
            buckets: vec![BucketConfig {
                name: "uploads".to_string(),
                path_prefix: "/uploads".to_string(),
                s3: S3Config {
                    bucket: "my-bucket".to_string(),
                    region: "eu-west-1".to_string(), // European region
                    endpoint: None, // Use default AWS endpoint
                    access_key: Some("AKIAIOSFODNN7EXAMPLE".to_string()),
                    secret_key: Some("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string()),
                },
                auth: AuthConfig::default(),
                upload: UploadConfig::default(),
            }],
            metrics: MetricsConfig::default(),
            tracing: None,
        };

        let pool = S3ClientPool::new(&config).await.unwrap();
        let client = pool.get_client("uploads").unwrap();

        // Client should be configured for eu-west-1 region
        assert_eq!(client.region(), "eu-west-1");
    }

    /// Test that pool clients use custom endpoint when provided
    #[tokio::test]
    async fn test_pool_client_custom_endpoint() {
        use mizuchi_uploadr::config::{
            AuthConfig, BucketConfig, Config, MetricsConfig, S3Config, ServerConfig,
            UploadConfig, ZeroCopyConfig,
        };
        use mizuchi_uploadr::s3::S3ClientPool;

        let config = Config {
            server: ServerConfig {
                address: "127.0.0.1:8080".to_string(),
                zero_copy: ZeroCopyConfig::default(),
            },
            buckets: vec![BucketConfig {
                name: "uploads".to_string(),
                path_prefix: "/uploads".to_string(),
                s3: S3Config {
                    bucket: "my-bucket".to_string(),
                    region: "us-east-1".to_string(),
                    endpoint: Some("http://localhost:9000".to_string()), // MinIO endpoint
                    access_key: Some("minioadmin".to_string()),
                    secret_key: Some("minioadmin".to_string()),
                },
                auth: AuthConfig::default(),
                upload: UploadConfig::default(),
            }],
            metrics: MetricsConfig::default(),
            tracing: None,
        };

        let pool = S3ClientPool::new(&config).await.unwrap();
        let client = pool.get_client("uploads").unwrap();

        // Client should use custom endpoint
        assert_eq!(client.endpoint(), "http://localhost:9000");
    }
}
