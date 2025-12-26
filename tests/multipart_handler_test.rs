//! Multipart Upload Handler Integration Tests
//!
//! Tests for the multipart upload handler with actual S3 client integration.
//! These tests follow TDD methodology - RED phase: tests are written before implementation.
//!
//! ## Test Coverage
//!
//! - Create multipart upload
//! - Upload parts with real ETags from S3
//! - Complete multipart upload
//! - Abort multipart upload
//! - Error handling for S3 failures
//! - Bucket mismatch validation

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use mizuchi_uploadr::s3::{S3Client, S3ClientConfig};
    use mizuchi_uploadr::upload::multipart::MultipartHandler;
    use wiremock::matchers::{body_string_contains, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Helper function to create an S3 client pointing to a mock server
    fn create_test_s3_client(mock_server: &MockServer, bucket: &str) -> S3Client {
        let s3_config = S3ClientConfig {
            bucket: bucket.to_string(),
            region: "us-east-1".to_string(),
            endpoint: Some(mock_server.uri()),
            access_key: Some("test-access".to_string()),
            secret_key: Some("test-secret".to_string()),
            retry: None,
            timeout: None,
        };
        S3Client::new(s3_config).unwrap()
    }

    // ========================================================================
    // TEST: Create Multipart Upload
    // ========================================================================

    /// Test that CreateMultipartUpload returns real upload_id from S3
    ///
    /// RED: This test will fail because MultipartHandler doesn't use S3Client yet
    #[tokio::test]
    async fn test_create_multipart_returns_real_upload_id() {
        let mock_server = MockServer::start().await;

        // Mock S3 CreateMultipartUpload response
        Mock::given(method("POST"))
            .and(path("/test-key.bin"))
            .and(query_param("uploads", ""))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"<?xml version="1.0" encoding="UTF-8"?>
                <InitiateMultipartUploadResult>
                    <Bucket>test-bucket</Bucket>
                    <Key>test-key.bin</Key>
                    <UploadId>real-upload-id-12345</UploadId>
                </InitiateMultipartUploadResult>"#,
            ))
            .expect(1)
            .mount(&mock_server)
            .await;

        let s3_client = create_test_s3_client(&mock_server, "test-bucket");

        // NOTE: This will fail until we add with_client() to MultipartHandler
        let handler = MultipartHandler::with_client(s3_client);

        let upload = handler.create("test-bucket", "test-key.bin").await.unwrap();

        // Verify real upload_id from S3 (not a UUID)
        assert_eq!(
            upload.upload_id, "real-upload-id-12345",
            "Should return real upload_id from S3, not a fake UUID"
        );
    }

    // ========================================================================
    // TEST: Upload Part
    // ========================================================================

    /// Test that UploadPart returns real ETag from S3
    #[tokio::test]
    async fn test_upload_part_returns_real_etag() {
        let mock_server = MockServer::start().await;

        // Mock CreateMultipartUpload
        Mock::given(method("POST"))
            .and(path("/test-key.bin"))
            .and(query_param("uploads", ""))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"<?xml version="1.0" encoding="UTF-8"?>
                <InitiateMultipartUploadResult>
                    <UploadId>upload-123</UploadId>
                </InitiateMultipartUploadResult>"#,
            ))
            .expect(1)
            .mount(&mock_server)
            .await;

        // Mock UploadPart response with specific ETag
        Mock::given(method("PUT"))
            .and(path("/test-key.bin"))
            .and(query_param("uploadId", "upload-123"))
            .and(query_param("partNumber", "1"))
            .respond_with(ResponseTemplate::new(200).insert_header("ETag", "\"part-etag-abc123\""))
            .expect(1)
            .mount(&mock_server)
            .await;

        let s3_client = create_test_s3_client(&mock_server, "test-bucket");
        let handler = MultipartHandler::with_client(s3_client);

        // Create upload first
        let mut upload = handler.create("test-bucket", "test-key.bin").await.unwrap();

        // Upload a part (5MB minimum)
        let body = Bytes::from(vec![b'x'; 5 * 1024 * 1024]);
        let part = handler.upload_part(&mut upload, 1, body).await.unwrap();

        assert_eq!(
            part.etag, "\"part-etag-abc123\"",
            "Should return real ETag from S3"
        );
        assert_eq!(part.part_number, 1);
    }

    /// Test uploading multiple parts
    #[tokio::test]
    async fn test_upload_multiple_parts() {
        let mock_server = MockServer::start().await;

        // Mock CreateMultipartUpload
        Mock::given(method("POST"))
            .and(path("/large-file.bin"))
            .and(query_param("uploads", ""))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"<InitiateMultipartUploadResult><UploadId>multi-upload</UploadId></InitiateMultipartUploadResult>"#,
            ))
            .mount(&mock_server)
            .await;

        // Mock UploadPart for part 1
        Mock::given(method("PUT"))
            .and(path("/large-file.bin"))
            .and(query_param("uploadId", "multi-upload"))
            .and(query_param("partNumber", "1"))
            .respond_with(ResponseTemplate::new(200).insert_header("ETag", "\"etag-part-1\""))
            .mount(&mock_server)
            .await;

        // Mock UploadPart for part 2
        Mock::given(method("PUT"))
            .and(path("/large-file.bin"))
            .and(query_param("uploadId", "multi-upload"))
            .and(query_param("partNumber", "2"))
            .respond_with(ResponseTemplate::new(200).insert_header("ETag", "\"etag-part-2\""))
            .mount(&mock_server)
            .await;

        let s3_client = create_test_s3_client(&mock_server, "test-bucket");
        let handler = MultipartHandler::with_client(s3_client);

        let mut upload = handler
            .create("test-bucket", "large-file.bin")
            .await
            .unwrap();

        // Upload two 5MB parts
        let body1 = Bytes::from(vec![b'a'; 5 * 1024 * 1024]);
        let part1 = handler.upload_part(&mut upload, 1, body1).await.unwrap();

        let body2 = Bytes::from(vec![b'b'; 5 * 1024 * 1024]);
        let part2 = handler.upload_part(&mut upload, 2, body2).await.unwrap();

        assert_eq!(part1.etag, "\"etag-part-1\"");
        assert_eq!(part2.etag, "\"etag-part-2\"");
        assert_eq!(upload.parts.len(), 2);
    }

    // ========================================================================
    // TEST: Complete Multipart Upload
    // ========================================================================

    /// Test that CompleteMultipartUpload returns final ETag from S3
    #[tokio::test]
    async fn test_complete_multipart_returns_final_etag() {
        let mock_server = MockServer::start().await;

        // Mock CreateMultipartUpload
        Mock::given(method("POST"))
            .and(path("/complete-test.bin"))
            .and(query_param("uploads", ""))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"<InitiateMultipartUploadResult><UploadId>complete-upload</UploadId></InitiateMultipartUploadResult>"#,
            ))
            .mount(&mock_server)
            .await;

        // Mock UploadPart
        Mock::given(method("PUT"))
            .and(path("/complete-test.bin"))
            .and(query_param("uploadId", "complete-upload"))
            .respond_with(ResponseTemplate::new(200).insert_header("ETag", "\"part-etag\""))
            .mount(&mock_server)
            .await;

        // Mock CompleteMultipartUpload
        Mock::given(method("POST"))
            .and(path("/complete-test.bin"))
            .and(query_param("uploadId", "complete-upload"))
            .and(body_string_contains("<CompleteMultipartUpload>"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"<?xml version="1.0" encoding="UTF-8"?>
                <CompleteMultipartUploadResult>
                    <Location>https://s3.amazonaws.com/test-bucket/complete-test.bin</Location>
                    <Bucket>test-bucket</Bucket>
                    <Key>complete-test.bin</Key>
                    <ETag>"final-etag-1-abc"</ETag>
                </CompleteMultipartUploadResult>"#,
            ))
            .expect(1)
            .mount(&mock_server)
            .await;

        let s3_client = create_test_s3_client(&mock_server, "test-bucket");
        let handler = MultipartHandler::with_client(s3_client);

        let mut upload = handler
            .create("test-bucket", "complete-test.bin")
            .await
            .unwrap();

        // Upload a part
        let body = Bytes::from(vec![b'x'; 5 * 1024 * 1024]);
        handler.upload_part(&mut upload, 1, body).await.unwrap();

        // Complete the upload
        let result = handler.complete(&upload).await.unwrap();

        assert_eq!(
            result.etag, "\"final-etag-1-abc\"",
            "Should return final ETag from S3"
        );
    }

    // ========================================================================
    // TEST: Abort Multipart Upload
    // ========================================================================

    /// Test that AbortMultipartUpload calls S3 correctly
    #[tokio::test]
    async fn test_abort_multipart_upload() {
        let mock_server = MockServer::start().await;

        // Mock CreateMultipartUpload
        Mock::given(method("POST"))
            .and(path("/abort-test.bin"))
            .and(query_param("uploads", ""))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"<InitiateMultipartUploadResult><UploadId>abort-upload</UploadId></InitiateMultipartUploadResult>"#,
            ))
            .mount(&mock_server)
            .await;

        // Mock AbortMultipartUpload (DELETE request)
        Mock::given(method("DELETE"))
            .and(path("/abort-test.bin"))
            .and(query_param("uploadId", "abort-upload"))
            .respond_with(ResponseTemplate::new(204))
            .expect(1)
            .mount(&mock_server)
            .await;

        let s3_client = create_test_s3_client(&mock_server, "test-bucket");
        let handler = MultipartHandler::with_client(s3_client);

        let upload = handler
            .create("test-bucket", "abort-test.bin")
            .await
            .unwrap();

        // Abort should call DELETE to S3
        let result = handler.abort(&upload).await;
        assert!(result.is_ok());
    }

    // ========================================================================
    // TEST: Error Handling
    // ========================================================================

    /// Test that CreateMultipartUpload errors are handled
    #[tokio::test]
    async fn test_create_multipart_handles_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/error-test.bin"))
            .and(query_param("uploads", ""))
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

        let s3_client = create_test_s3_client(&mock_server, "test-bucket");
        let handler = MultipartHandler::with_client(s3_client);

        let result = handler.create("test-bucket", "error-test.bin").await;

        assert!(result.is_err(), "Should return error for 403 response");
    }

    /// Test that UploadPart errors are handled
    #[tokio::test]
    async fn test_upload_part_handles_error() {
        let mock_server = MockServer::start().await;

        // Mock successful CreateMultipartUpload
        Mock::given(method("POST"))
            .and(path("/part-error.bin"))
            .and(query_param("uploads", ""))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"<InitiateMultipartUploadResult><UploadId>part-error-upload</UploadId></InitiateMultipartUploadResult>"#,
            ))
            .mount(&mock_server)
            .await;

        // Mock failing UploadPart
        Mock::given(method("PUT"))
            .and(path("/part-error.bin"))
            .and(query_param("uploadId", "part-error-upload"))
            .respond_with(
                ResponseTemplate::new(500)
                    .set_body_string(r#"<Error><Code>InternalError</Code></Error>"#),
            )
            .mount(&mock_server)
            .await;

        let s3_client = create_test_s3_client(&mock_server, "test-bucket");
        let handler = MultipartHandler::with_client(s3_client);

        let mut upload = handler
            .create("test-bucket", "part-error.bin")
            .await
            .unwrap();

        let body = Bytes::from(vec![b'x'; 5 * 1024 * 1024]);
        let result = handler.upload_part(&mut upload, 1, body).await;

        assert!(result.is_err(), "Should return error for 500 response");
    }

    // ========================================================================
    // TEST: Bucket Validation
    // ========================================================================

    /// Test that bucket mismatch is rejected
    #[tokio::test]
    async fn test_create_rejects_bucket_mismatch() {
        let mock_server = MockServer::start().await;

        // Client configured for "configured-bucket"
        let s3_client = create_test_s3_client(&mock_server, "configured-bucket");
        let handler = MultipartHandler::with_client(s3_client);

        // But we try to create upload for "different-bucket"
        let result = handler.create("different-bucket", "test-key").await;

        assert!(result.is_err(), "Should return error for bucket mismatch");
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("Bucket mismatch"),
            "Error should indicate bucket mismatch: {}",
            err
        );
    }
}
