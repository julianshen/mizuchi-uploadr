//! PutObject Handler Integration Tests
//!
//! Tests for the PutObject upload handler with actual S3 client integration.
//! These tests follow TDD methodology - RED phase: tests are written before implementation.
//!
//! # Task 9: Phase 2.1 - Simple PutObject Handler
//!
//! ## Test Coverage
//!
//! - Upload small files (1MB) through handler to S3
//! - Upload medium files (up to 50MB threshold)
//! - Error handling for S3 failures
//! - Content-Type preservation
//! - ETag verification (real S3 response, not fake UUID)

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use mizuchi_uploadr::s3::{S3Client, S3ClientConfig};
    use mizuchi_uploadr::upload::put_object::PutObjectHandler;
    use mizuchi_uploadr::upload::UploadHandler;
    use wiremock::matchers::{body_bytes, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // ========================================================================
    // TEST: Small File Upload (1MB)
    // ========================================================================

    /// Test that PutObjectHandler uploads a small file to S3 and returns real ETag
    ///
    /// This test verifies that:
    /// - Handler connects to S3Client
    /// - Request is made to the correct S3 endpoint
    /// - Real ETag from S3 response is returned (not a fake UUID)
    #[tokio::test]
    async fn test_upload_small_file_returns_real_etag() {
        let mock_server = MockServer::start().await;

        // Mock S3 PutObject response with specific ETag
        Mock::given(method("PUT"))
            .and(path("/test-key.txt"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("ETag", "\"d41d8cd98f00b204e9800998ecf8427e\""),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        // Create S3 client pointing to mock server
        let s3_config = S3ClientConfig {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            endpoint: Some(mock_server.uri()),
            access_key: Some("test-access".to_string()),
            secret_key: Some("test-secret".to_string()),
            retry: None,
            timeout: None,
        };
        let s3_client = S3Client::new(s3_config).unwrap();

        // Create handler with S3 client
        // NOTE: This will fail until we modify PutObjectHandler to accept S3Client
        let handler = PutObjectHandler::with_client(s3_client);

        // Upload small file (1KB for test)
        let body = Bytes::from(vec![b'x'; 1024]);
        let result = handler
            .upload("test-bucket", "test-key.txt", body, Some("text/plain"))
            .await
            .unwrap();

        // Verify real ETag from S3 (not a UUID)
        assert_eq!(
            result.etag, "\"d41d8cd98f00b204e9800998ecf8427e\"",
            "Should return real ETag from S3, not a fake UUID"
        );
        assert_eq!(result.bytes_written, 1024);
    }

    /// Test upload with 1MB file size
    #[tokio::test]
    async fn test_upload_1mb_file() {
        let mock_server = MockServer::start().await;

        Mock::given(method("PUT"))
            .and(path("/large-file.bin"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("ETag", "\"1mb-file-etag-12345\""),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let s3_config = S3ClientConfig {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            endpoint: Some(mock_server.uri()),
            access_key: Some("test-access".to_string()),
            secret_key: Some("test-secret".to_string()),
            retry: None,
            timeout: None,
        };
        let s3_client = S3Client::new(s3_config).unwrap();

        let handler = PutObjectHandler::with_client(s3_client);

        // Create 1MB body
        let body = Bytes::from(vec![b'a'; 1024 * 1024]);
        let result = handler
            .upload("test-bucket", "large-file.bin", body, Some("application/octet-stream"))
            .await
            .unwrap();

        assert_eq!(result.etag, "\"1mb-file-etag-12345\"");
        assert_eq!(result.bytes_written, 1024 * 1024);
    }

    // ========================================================================
    // TEST: Content-Type Handling
    // ========================================================================

    /// Test that Content-Type header is passed through to S3
    #[tokio::test]
    async fn test_upload_preserves_content_type() {
        let mock_server = MockServer::start().await;

        // Verify Content-Type header is sent
        Mock::given(method("PUT"))
            .and(path("/document.json"))
            .and(header("Content-Type", "application/json"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("ETag", "\"json-etag\""),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let s3_config = S3ClientConfig {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            endpoint: Some(mock_server.uri()),
            access_key: Some("test-access".to_string()),
            secret_key: Some("test-secret".to_string()),
            retry: None,
            timeout: None,
        };
        let s3_client = S3Client::new(s3_config).unwrap();

        let handler = PutObjectHandler::with_client(s3_client);

        let body = Bytes::from(r#"{"key": "value"}"#);
        let result = handler
            .upload("test-bucket", "document.json", body, Some("application/json"))
            .await
            .unwrap();

        assert_eq!(result.etag, "\"json-etag\"");
    }

    /// Test upload without Content-Type (should use default)
    #[tokio::test]
    async fn test_upload_without_content_type() {
        let mock_server = MockServer::start().await;

        Mock::given(method("PUT"))
            .and(path("/binary-file"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("ETag", "\"binary-etag\""),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let s3_config = S3ClientConfig {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            endpoint: Some(mock_server.uri()),
            access_key: Some("test-access".to_string()),
            secret_key: Some("test-secret".to_string()),
            retry: None,
            timeout: None,
        };
        let s3_client = S3Client::new(s3_config).unwrap();

        let handler = PutObjectHandler::with_client(s3_client);

        let body = Bytes::from(vec![0u8; 256]);
        let result = handler
            .upload("test-bucket", "binary-file", body, None) // No content type
            .await
            .unwrap();

        assert_eq!(result.etag, "\"binary-etag\"");
    }

    // ========================================================================
    // TEST: Error Handling
    // ========================================================================

    /// Test that S3 errors are properly propagated
    #[tokio::test]
    async fn test_upload_handles_s3_access_denied() {
        let mock_server = MockServer::start().await;

        Mock::given(method("PUT"))
            .and(path("/forbidden-key"))
            .respond_with(ResponseTemplate::new(403).set_body_string(
                r#"<?xml version="1.0" encoding="UTF-8"?>
                <Error>
                    <Code>AccessDenied</Code>
                    <Message>Access Denied</Message>
                </Error>"#,
            ))
            .expect(1)
            .mount(&mock_server)
            .await;

        let s3_config = S3ClientConfig {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            endpoint: Some(mock_server.uri()),
            access_key: Some("test-access".to_string()),
            secret_key: Some("test-secret".to_string()),
            retry: None,
            timeout: None,
        };
        let s3_client = S3Client::new(s3_config).unwrap();

        let handler = PutObjectHandler::with_client(s3_client);

        let body = Bytes::from("test data");
        let result = handler
            .upload("test-bucket", "forbidden-key", body, None)
            .await;

        assert!(result.is_err(), "Should return error for 403 response");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("403") || err.to_string().contains("AccessDenied"),
            "Error should indicate access denied"
        );
    }

    /// Test that S3 server errors are handled
    #[tokio::test]
    async fn test_upload_handles_s3_server_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("PUT"))
            .and(path("/error-key"))
            .respond_with(ResponseTemplate::new(500).set_body_string(
                r#"<?xml version="1.0" encoding="UTF-8"?>
                <Error>
                    <Code>InternalError</Code>
                    <Message>Internal Server Error</Message>
                </Error>"#,
            ))
            // Note: With retry logic, this might be called multiple times
            .mount(&mock_server)
            .await;

        let s3_config = S3ClientConfig {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            endpoint: Some(mock_server.uri()),
            access_key: Some("test-access".to_string()),
            secret_key: Some("test-secret".to_string()),
            // Use minimal retries for faster test
            retry: Some(mizuchi_uploadr::s3::RetryConfig {
                max_retries: 1,
                initial_backoff_ms: 10,
                max_backoff_ms: 100,
                backoff_multiplier: 2.0,
            }),
            timeout: None,
        };
        let s3_client = S3Client::new(s3_config).unwrap();

        let handler = PutObjectHandler::with_client(s3_client);

        let body = Bytes::from("test data");
        let result = handler
            .upload("test-bucket", "error-key", body, None)
            .await;

        assert!(result.is_err(), "Should return error for 500 response");
    }

    // ========================================================================
    // TEST: Body Integrity
    // ========================================================================

    /// Test that the exact body is sent to S3
    #[tokio::test]
    async fn test_upload_sends_exact_body() {
        let mock_server = MockServer::start().await;

        let expected_body = b"Hello, S3 World!";

        // Verify exact body bytes are received
        Mock::given(method("PUT"))
            .and(path("/exact-body-test"))
            .and(body_bytes(expected_body.to_vec()))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("ETag", "\"exact-body-etag\""),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let s3_config = S3ClientConfig {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            endpoint: Some(mock_server.uri()),
            access_key: Some("test-access".to_string()),
            secret_key: Some("test-secret".to_string()),
            retry: None,
            timeout: None,
        };
        let s3_client = S3Client::new(s3_config).unwrap();

        let handler = PutObjectHandler::with_client(s3_client);

        let body = Bytes::from(&expected_body[..]);
        let result = handler
            .upload("test-bucket", "exact-body-test", body, Some("text/plain"))
            .await
            .unwrap();

        assert_eq!(result.etag, "\"exact-body-etag\"");
    }
}
