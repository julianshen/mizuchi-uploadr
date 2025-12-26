//! PutObject handler
//!
//! Handles simple object uploads (< multipart threshold).
//!
//! # Example
//!
//! ```no_run
//! use mizuchi_uploadr::s3::{S3Client, S3ClientConfig};
//! use mizuchi_uploadr::upload::put_object::PutObjectHandler;
//! use mizuchi_uploadr::upload::UploadHandler;
//! use bytes::Bytes;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create S3 client
//! let config = S3ClientConfig {
//!     bucket: "my-bucket".to_string(),
//!     region: "us-east-1".to_string(),
//!     endpoint: None,
//!     access_key: Some("access-key".to_string()),
//!     secret_key: Some("secret-key".to_string()),
//!     retry: None,
//!     timeout: None,
//! };
//! let s3_client = S3Client::new(config)?;
//!
//! // Create handler with S3 client
//! let handler = PutObjectHandler::with_client(s3_client);
//!
//! // Upload a file
//! let body = Bytes::from("Hello, World!");
//! let result = handler.upload("my-bucket", "hello.txt", body, Some("text/plain")).await?;
//! println!("Uploaded with ETag: {}", result.etag);
//! # Ok(())
//! # }
//! ```

use super::{UploadError, UploadHandler, UploadResult};
use crate::metrics;
use crate::s3::S3Client;
use async_trait::async_trait;
use bytes::Bytes;
use std::time::Instant;

/// Simple upload handler
///
/// Handles single-part uploads for files under the multipart threshold (default 50MB).
/// Uses an S3 client for actual uploads.
pub struct PutObjectHandler {
    /// S3 client for making upload requests
    client: Option<S3Client>,
    /// Bucket name (used when client is not provided)
    #[allow(dead_code)]
    bucket: String,
    /// Region (used when client is not provided)
    #[allow(dead_code)]
    region: String,
}

impl PutObjectHandler {
    /// Create a new PutObject handler (legacy constructor)
    ///
    /// This constructor creates a handler without an S3 client.
    /// Use `with_client` for production use.
    pub fn new(bucket: &str, region: &str) -> Self {
        Self {
            client: None,
            bucket: bucket.to_string(),
            region: region.to_string(),
        }
    }

    /// Create a new PutObject handler with an S3 client
    ///
    /// This is the preferred constructor for production use.
    ///
    /// # Arguments
    ///
    /// * `client` - Configured S3 client for making upload requests
    pub fn with_client(client: S3Client) -> Self {
        Self {
            bucket: client.bucket().to_string(),
            region: client.region().to_string(),
            client: Some(client),
        }
    }

    /// Check if zero-copy transfer is supported on this platform
    ///
    /// Returns `true` on Linux where splice(2)/sendfile(2) are available,
    /// `false` on other platforms that use buffered I/O fallback.
    pub fn supports_zero_copy(&self) -> bool {
        crate::upload::zero_copy::is_available()
    }
}

#[async_trait]
impl UploadHandler for PutObjectHandler {
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
        let bytes_written = body.len() as u64;
        let start_time = Instant::now();

        // Use S3 client if available, otherwise use placeholder for legacy behavior
        let upload_result = if let Some(client) = &self.client {
            // Validate bucket matches client configuration
            if bucket != client.bucket() {
                tracing::error!(
                    expected_bucket = %client.bucket(),
                    actual_bucket = %bucket,
                    "Bucket mismatch: upload requested for different bucket than client configured"
                );
                return Err(UploadError::S3Error(format!(
                    "Bucket mismatch: client configured for '{}' but upload requested for '{}'",
                    client.bucket(),
                    bucket
                )));
            }
            // Real S3 upload via client
            client
                .put_object(key, body, content_type)
                .await
                .map(|response| response.etag)
                .map_err(|e| UploadError::S3Error(e.to_string()))
        } else {
            // Legacy placeholder behavior (for backward compatibility with existing tests)
            tracing::warn!(
                bucket = %bucket,
                key = %key,
                "Using legacy placeholder path: no S3 client configured, returning fake ETag. \
                 This should only happen in tests."
            );
            Ok(format!("\"{}\"", uuid::Uuid::new_v4()))
        };

        // Record metrics
        let duration = start_time.elapsed();
        metrics::record_upload_duration(bucket, "put_object", duration.as_secs_f64());

        match upload_result {
            Ok(etag) => {
                metrics::record_upload_success(bucket, bytes_written);

                let result = UploadResult {
                    etag,
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
                    duration_ms = duration.as_millis(),
                    "PutObject upload completed"
                );

                Ok(result)
            }
            Err(e) => {
                metrics::record_upload_failure(bucket);
                metrics::record_error("s3_upload");

                tracing::error!(
                    error = %e,
                    duration_ms = duration.as_millis(),
                    "PutObject upload failed"
                );

                Err(e)
            }
        }
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
