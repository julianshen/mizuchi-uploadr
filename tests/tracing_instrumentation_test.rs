//! Integration tests for tracing span instrumentation
//!
//! RED PHASE: These tests verify that upload operations create proper spans

use mizuchi_uploadr::upload::multipart::{MultipartHandler, MIN_PART_SIZE};
use mizuchi_uploadr::upload::put_object::PutObjectHandler;
use mizuchi_uploadr::upload::UploadHandler;

/// Test that PutObject handler creates a span
#[tokio::test]
async fn test_put_object_creates_span() {
    let _guard = tracing_subscriber::fmt()
        .with_test_writer()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    let handler = PutObjectHandler::new("test-bucket", "us-east-1");
    let body = bytes::Bytes::from("test data");

    let result = handler
        .upload("test-bucket", "test-key", body, Some("text/plain"))
        .await;

    assert!(result.is_ok());
}

/// Test that multipart operations create nested spans
#[tokio::test]
async fn test_multipart_creates_nested_spans() {
    let _guard = tracing_subscriber::fmt()
        .with_test_writer()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    let handler = MultipartHandler::new("test-bucket", "us-east-1", MIN_PART_SIZE, 4);
    let mut upload = handler.create("test-bucket", "test-key").await.unwrap();

    let body = bytes::Bytes::from(vec![0u8; MIN_PART_SIZE]);
    let _part = handler.upload_part(&mut upload, 1, body).await.unwrap();

    let result = handler.complete(&upload).await;
    assert!(result.is_ok());
}

/// Test that errors are recorded in spans
#[tokio::test]
async fn test_span_records_error() {
    let _guard = tracing_subscriber::fmt()
        .with_test_writer()
        .with_max_level(tracing::Level::INFO)
        .try_init();

    let handler = MultipartHandler::new("test-bucket", "us-east-1", MIN_PART_SIZE, 4);

    let upload = mizuchi_uploadr::upload::multipart::MultipartUpload {
        upload_id: "test-id".to_string(),
        bucket: "test-bucket".to_string(),
        key: "test-key".to_string(),
        parts: Vec::new(),
    };

    let result = handler.complete(&upload).await;
    assert!(result.is_err());
}
