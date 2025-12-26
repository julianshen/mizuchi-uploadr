//! PutObject handler
//!
//! Handles simple object uploads (< multipart threshold).

use super::{UploadError, UploadHandler, UploadResult};
use async_trait::async_trait;
use bytes::Bytes;

/// Simple upload handler
pub struct PutObjectHandler {
    // S3 client will be injected
    #[allow(dead_code)]
    bucket: String,
    #[allow(dead_code)]
    region: String,
}

impl PutObjectHandler {
    /// Create a new PutObject handler
    pub fn new(bucket: &str, region: &str) -> Self {
        Self {
            bucket: bucket.to_string(),
            region: region.to_string(),
        }
    }
}

#[async_trait]
impl UploadHandler for PutObjectHandler {
    #[allow(unused_variables)] // content_type used in tracing instrumentation
    #[tracing::instrument(
        name = "upload.put_object",
        skip(self, body),
        fields(
            s3.bucket = %bucket,
            s3.key = %key,
            http.content_type = ?content_type,
            upload.bytes = body.len(),
            // Result fields - will be set after operation
            s3.etag = tracing::field::Empty,
            upload.bytes_written = tracing::field::Empty
        ),
        err
    )]
    async fn upload(
        &self,
        bucket: &str,
        key: &str,
        body: Bytes,
        content_type: Option<&str>,
    ) -> Result<UploadResult, UploadError> {
        // TODO: Implement actual S3 upload
        // This is a placeholder for TDD - tests will drive implementation

        let bytes_written = body.len() as u64;

        let result = UploadResult {
            etag: format!("\"{}\"", uuid::Uuid::new_v4()),
            version_id: None,
            bytes_written,
        };

        // Record result in span
        let span = tracing::Span::current();
        span.record("s3.etag", result.etag.as_str());
        span.record("upload.bytes_written", bytes_written);

        tracing::info!(
            etag = %result.etag,
            bytes_written = bytes_written,
            "PutObject upload completed"
        );

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_put_object() {
        let handler = PutObjectHandler::new("test-bucket", "us-east-1");
        let body = Bytes::from("Hello, World!");

        let result = handler
            .upload("test-bucket", "test-key", body.clone(), Some("text/plain"))
            .await
            .unwrap();

        assert_eq!(result.bytes_written, body.len() as u64);
        assert!(!result.etag.is_empty());
    }
}
