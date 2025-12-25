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

        tracing::info!(
            bucket = bucket,
            key = key,
            size = bytes_written,
            content_type = content_type,
            "PutObject upload"
        );

        Ok(UploadResult {
            etag: format!("\"{}\"", uuid::Uuid::new_v4()),
            version_id: None,
            bytes_written,
        })
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
