//! S3 HTTP API Integration Tests
//!
//! Tests for S3 client HTTP requests with SigV4 signing and trace context.
//! These tests verify that actual HTTP requests are made to S3 endpoints.

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use mizuchi_uploadr::s3::{S3Client, S3ClientConfig, S3CompletedPart};
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Helper to create S3 client config for testing
    fn create_test_config(endpoint: String) -> S3ClientConfig {
        S3ClientConfig {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            endpoint: Some(endpoint),
            access_key: Some("AKIAIOSFODNN7EXAMPLE".to_string()),
            secret_key: Some("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string()),
        }
    }

    #[tokio::test]
    async fn test_put_object_makes_http_request() {
        // Start mock S3 server
        let mock_server = MockServer::start().await;

        // Mock S3 PutObject response
        Mock::given(method("PUT"))
            .and(path("/test-key"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("ETag", "\"abc123\"")
                    .insert_header("x-amz-request-id", "test-request-id"),
            )
            .expect(1)
            .mount(&mock_server)
            .await;

        let config = create_test_config(mock_server.uri());
        let client = S3Client::new(config).unwrap();

        let body = Bytes::from("test data");
        let response = client
            .put_object("test-key", body, Some("text/plain"))
            .await
            .unwrap();

        assert_eq!(response.etag, "\"abc123\"");
    }

    #[tokio::test]
    async fn test_put_object_with_different_content_types() {
        let mock_server = MockServer::start().await;

        Mock::given(method("PUT"))
            .and(path("/test-key.json"))
            .respond_with(ResponseTemplate::new(200).insert_header("ETag", "\"json-etag\""))
            .expect(1)
            .mount(&mock_server)
            .await;

        let config = create_test_config(mock_server.uri());
        let client = S3Client::new(config).unwrap();

        let body = Bytes::from(r#"{"key": "value"}"#);
        let response = client
            .put_object("test-key.json", body, Some("application/json"))
            .await
            .unwrap();

        assert_eq!(response.etag, "\"json-etag\"");
    }

    #[tokio::test]
    async fn test_create_multipart_upload_makes_http_request() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/test-key"))
            .and(query_param("uploads", ""))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"<?xml version="1.0" encoding="UTF-8"?>
                    <InitiateMultipartUploadResult>
                        <UploadId>test-upload-id-123</UploadId>
                    </InitiateMultipartUploadResult>"#,
            ))
            .expect(1)
            .mount(&mock_server)
            .await;

        let config = create_test_config(mock_server.uri());
        let client = S3Client::new(config).unwrap();

        let response = client.create_multipart_upload("test-key").await.unwrap();
        assert_eq!(response.upload_id, "test-upload-id-123");
    }

    #[tokio::test]
    async fn test_upload_part_makes_http_request() {
        let mock_server = MockServer::start().await;

        Mock::given(method("PUT"))
            .and(path("/test-key"))
            .and(query_param("partNumber", "1"))
            .and(query_param("uploadId", "test-upload-id"))
            .respond_with(ResponseTemplate::new(200).insert_header("ETag", "\"part-etag-1\""))
            .expect(1)
            .mount(&mock_server)
            .await;

        let config = create_test_config(mock_server.uri());
        let client = S3Client::new(config).unwrap();

        let body = Bytes::from("part data");
        let response = client.upload_part("test-upload-id", 1, body).await.unwrap();

        assert_eq!(response.etag, "\"part-etag-1\"");
    }

    #[tokio::test]
    async fn test_complete_multipart_upload_makes_http_request() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/test-key"))
            .and(query_param("uploadId", "test-upload-id"))
            .respond_with(ResponseTemplate::new(200).set_body_string(
                r#"<?xml version="1.0" encoding="UTF-8"?>
                    <CompleteMultipartUploadResult>
                        <ETag>"final-etag-123"</ETag>
                    </CompleteMultipartUploadResult>"#,
            ))
            .expect(1)
            .mount(&mock_server)
            .await;

        let config = create_test_config(mock_server.uri());
        let client = S3Client::new(config).unwrap();

        let parts = vec![
            S3CompletedPart {
                part_number: 1,
                etag: "\"part-etag-1\"".to_string(),
            },
            S3CompletedPart {
                part_number: 2,
                etag: "\"part-etag-2\"".to_string(),
            },
        ];

        let response = client
            .complete_multipart_upload("test-upload-id", parts)
            .await
            .unwrap();

        assert_eq!(response.etag, "\"final-etag-123\"");
    }

    #[tokio::test]
    async fn test_s3_error_handling() {
        let mock_server = MockServer::start().await;

        Mock::given(method("PUT"))
            .and(path("/test-key"))
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

        let config = create_test_config(mock_server.uri());
        let client = S3Client::new(config).unwrap();

        let body = Bytes::from("test data");
        let result = client
            .put_object("test-key", body, Some("text/plain"))
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("403") || err.to_string().contains("AccessDenied"));
    }
}
