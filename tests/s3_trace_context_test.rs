//! S3 W3C Trace Context Integration Tests
//!
//! Tests for W3C Trace Context propagation in S3 client HTTP requests.
//! Verifies that traceparent and tracestate headers are injected into S3 requests.

#[cfg(test)]
mod tests {
    use bytes::Bytes;
    use mizuchi_uploadr::s3::{S3Client, S3ClientConfig};
    use wiremock::matchers::{header_exists, header_regex, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Helper to create S3 client config for testing
    fn create_test_config(endpoint: String) -> S3ClientConfig {
        S3ClientConfig {
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            endpoint: Some(endpoint),
            access_key: Some("AKIAIOSFODNN7EXAMPLE".to_string()),
            secret_key: Some("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string()),
            retry: None,
            timeout: None,
        }
    }

    #[tokio::test]
    async fn test_put_object_injects_traceparent_header() {
        let mock_server = MockServer::start().await;

        // Expect traceparent header to be present
        Mock::given(method("PUT"))
            .and(path("/test-key"))
            .and(header_exists("traceparent"))
            .respond_with(ResponseTemplate::new(200).insert_header("ETag", "\"abc123\""))
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
    async fn test_put_object_injects_valid_traceparent_format() {
        let mock_server = MockServer::start().await;

        // Expect traceparent header with valid W3C format
        // Format: 00-{trace-id}-{span-id}-{flags}
        Mock::given(method("PUT"))
            .and(path("/test-key"))
            .and(header_regex(
                "traceparent",
                r"^00-[0-9a-f]{32}-[0-9a-f]{16}-[0-9a-f]{2}$",
            ))
            .respond_with(ResponseTemplate::new(200).insert_header("ETag", "\"abc123\""))
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
    async fn test_create_multipart_upload_injects_traceparent() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/test-key"))
            .and(header_exists("traceparent"))
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
    async fn test_upload_part_injects_traceparent() {
        let mock_server = MockServer::start().await;

        Mock::given(method("PUT"))
            .and(path("/test-key"))
            .and(header_exists("traceparent"))
            .respond_with(ResponseTemplate::new(200).insert_header("ETag", "\"part-etag-1\""))
            .expect(1)
            .mount(&mock_server)
            .await;

        let config = create_test_config(mock_server.uri());
        let client = S3Client::new(config).unwrap();

        let body = Bytes::from("part data");
        let response = client
            .upload_part("test-key", "test-upload-id", 1, body)
            .await
            .unwrap();

        assert_eq!(response.etag, "\"part-etag-1\"");
    }

    #[tokio::test]
    async fn test_complete_multipart_upload_injects_traceparent() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/test-key"))
            .and(header_exists("traceparent"))
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

        let parts = vec![mizuchi_uploadr::s3::S3CompletedPart {
            part_number: 1,
            etag: "\"part-etag-1\"".to_string(),
        }];

        let response = client
            .complete_multipart_upload("test-key", "test-upload-id", parts)
            .await
            .unwrap();

        assert_eq!(response.etag, "\"final-etag-123\"");
    }
}
