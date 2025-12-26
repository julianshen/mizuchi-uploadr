//! Multipart upload handler
//!
//! Handles large file uploads using S3 multipart upload API.
//!
//! # Example
//!
//! ```no_run
//! use mizuchi_uploadr::s3::{S3Client, S3ClientConfig};
//! use mizuchi_uploadr::upload::multipart::MultipartHandler;
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
//! let handler = MultipartHandler::with_client(s3_client);
//!
//! // Create multipart upload
//! let mut upload = handler.create("my-bucket", "large-file.bin").await?;
//!
//! // Upload parts
//! let part1 = Bytes::from(vec![0u8; 5 * 1024 * 1024]);
//! handler.upload_part(&mut upload, 1, part1).await?;
//!
//! // Complete upload
//! let result = handler.complete(&upload).await?;
//! println!("Uploaded with ETag: {}", result.etag);
//! # Ok(())
//! # }
//! ```

use super::{UploadError, UploadResult};
use crate::metrics::{record_multipart_upload_failure, record_multipart_upload_success};
use crate::s3::{S3Client, S3CompletedPart};
use bytes::Bytes;

/// Minimum part size (5MB) - S3 requirement
pub const MIN_PART_SIZE: usize = 5 * 1024 * 1024;

/// Maximum parts allowed
pub const MAX_PARTS: usize = 10000;

/// Multipart upload state
#[derive(Debug)]
pub struct MultipartUpload {
    pub upload_id: String,
    pub bucket: String,
    pub key: String,
    pub parts: Vec<CompletedPart>,
}

/// Completed part info
#[derive(Debug, Clone)]
pub struct CompletedPart {
    pub part_number: u32,
    pub etag: String,
}

/// Multipart upload handler
pub struct MultipartHandler {
    /// S3 client for making upload requests
    client: Option<S3Client>,
    /// Bucket name (used when client is not provided)
    #[allow(dead_code)]
    bucket: String,
    /// Region (used when client is not provided)
    #[allow(dead_code)]
    region: String,
    /// Part size for splitting uploads
    #[allow(dead_code)]
    part_size: usize,
    /// Number of concurrent part uploads
    #[allow(dead_code)]
    concurrent_parts: usize,
}

impl MultipartHandler {
    /// Create a new multipart handler (legacy constructor)
    ///
    /// This constructor creates a handler without an S3 client.
    /// Use `with_client` for production use.
    pub fn new(bucket: &str, region: &str, part_size: usize, concurrent_parts: usize) -> Self {
        Self {
            client: None,
            bucket: bucket.to_string(),
            region: region.to_string(),
            part_size: std::cmp::max(part_size, MIN_PART_SIZE),
            concurrent_parts,
        }
    }

    /// Create a new multipart handler with an S3 client
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
            part_size: MIN_PART_SIZE,
            concurrent_parts: 4,
        }
    }

    /// Check if zero-copy transfer is supported on this platform
    ///
    /// Returns `true` on Linux where splice(2)/sendfile(2) are available,
    /// `false` on other platforms that use buffered I/O fallback.
    pub fn supports_zero_copy(&self) -> bool {
        crate::upload::zero_copy::is_available()
    }

    /// Initiate a multipart upload
    #[tracing::instrument(
        name = "upload.multipart.create",
        skip(self),
        fields(
            s3.bucket = %bucket,
            s3.key = %key,
            upload_id = tracing::field::Empty
        ),
        err
    )]
    pub async fn create(&self, bucket: &str, key: &str) -> Result<MultipartUpload, UploadError> {
        // Use S3Client if available
        if let Some(client) = &self.client {
            // Validate bucket matches client configuration
            if bucket != client.bucket() {
                return Err(UploadError::BucketMismatch {
                    expected: client.bucket().to_string(),
                    actual: bucket.to_string(),
                });
            }

            // Call S3 CreateMultipartUpload API
            let response = client.create_multipart_upload(key).await?;

            // Record upload_id in span
            tracing::Span::current().record("upload_id", response.upload_id.as_str());

            tracing::info!(
                upload_id = %response.upload_id,
                "Created multipart upload"
            );

            return Ok(MultipartUpload {
                upload_id: response.upload_id,
                bucket: bucket.to_string(),
                key: key.to_string(),
                parts: Vec::new(),
            });
        }

        // Legacy mode: generate UUID (for backward compatibility)
        let upload_id = uuid::Uuid::new_v4().to_string();

        // Record upload_id in span
        tracing::Span::current().record("upload_id", upload_id.as_str());

        tracing::info!(
            upload_id = %upload_id,
            "Created multipart upload (legacy mode)"
        );

        Ok(MultipartUpload {
            upload_id,
            bucket: bucket.to_string(),
            key: key.to_string(),
            parts: Vec::new(),
        })
    }

    /// Upload a part
    #[tracing::instrument(
        name = "upload.multipart.upload_part",
        skip(self, upload, body),
        fields(
            upload_id = %upload.upload_id,
            part_number = part_number,
            upload.bytes = body.len(),
            s3.etag = tracing::field::Empty
        ),
        err
    )]
    pub async fn upload_part(
        &self,
        upload: &mut MultipartUpload,
        part_number: u32,
        body: Bytes,
    ) -> Result<CompletedPart, UploadError> {
        if body.len() < MIN_PART_SIZE && part_number < MAX_PARTS as u32 {
            // Allow smaller final part
            tracing::warn!(
                part_number = part_number,
                size = body.len(),
                "Part may be too small (< 5MB)"
            );
        }

        // Use S3Client if available
        if let Some(client) = &self.client {
            let response = client
                .upload_part(&upload.key, &upload.upload_id, part_number, body.clone())
                .await?;

            let part = CompletedPart {
                part_number,
                etag: response.etag.clone(),
            };

            upload.parts.push(part.clone());

            // Record etag in span
            tracing::Span::current().record("s3.etag", part.etag.as_str());

            tracing::info!(
                etag = %part.etag,
                size = body.len(),
                "Uploaded part"
            );

            return Ok(part);
        }

        // Legacy mode: generate fake ETag
        let etag = format!("\"part-{}\"", uuid::Uuid::new_v4());

        let part = CompletedPart {
            part_number,
            etag: etag.clone(),
        };

        upload.parts.push(part.clone());

        // Record etag in span
        tracing::Span::current().record("s3.etag", part.etag.as_str());

        tracing::info!(
            etag = %part.etag,
            size = body.len(),
            "Uploaded part (legacy mode)"
        );

        Ok(part)
    }

    /// Complete a multipart upload
    #[tracing::instrument(
        name = "upload.multipart.complete",
        skip(self, upload),
        fields(
            upload_id = %upload.upload_id,
            parts_count = upload.parts.len(),
            s3.etag = tracing::field::Empty
        ),
        err
    )]
    pub async fn complete(&self, upload: &MultipartUpload) -> Result<UploadResult, UploadError> {
        if upload.parts.is_empty() {
            return Err(UploadError::MultipartError("No parts uploaded".into()));
        }

        // Use S3Client if available
        if let Some(client) = &self.client {
            // Convert our CompletedPart to S3CompletedPart
            let s3_parts: Vec<S3CompletedPart> = upload
                .parts
                .iter()
                .map(|p| S3CompletedPart {
                    part_number: p.part_number,
                    etag: p.etag.clone(),
                })
                .collect();

            let response = client
                .complete_multipart_upload(&upload.key, &upload.upload_id, s3_parts)
                .await?;

            let result = UploadResult {
                etag: response.etag,
                version_id: None,
                bytes_written: 0, // S3 doesn't return this in CompleteMultipartUpload
            };

            // Record etag in span
            tracing::Span::current().record("s3.etag", result.etag.as_str());

            tracing::info!(
                etag = %result.etag,
                parts = upload.parts.len(),
                "Completed multipart upload"
            );

            // Record success metrics
            record_multipart_upload_success(&upload.bucket, upload.parts.len());

            return Ok(result);
        }

        // Legacy mode: generate fake final ETag
        let result = UploadResult {
            etag: format!("\"{}-{}\"", uuid::Uuid::new_v4(), upload.parts.len()),
            version_id: None,
            bytes_written: 0,
        };

        // Record etag in span
        tracing::Span::current().record("s3.etag", result.etag.as_str());

        tracing::info!(
            etag = %result.etag,
            parts = upload.parts.len(),
            "Completed multipart upload (legacy mode)"
        );

        // Record success metrics (legacy mode)
        record_multipart_upload_success(&upload.bucket, upload.parts.len());

        Ok(result)
    }

    /// Abort a multipart upload
    #[tracing::instrument(
        name = "upload.multipart.abort",
        skip(self, upload),
        fields(
            upload_id = %upload.upload_id,
            s3.key = %upload.key
        ),
        err
    )]
    pub async fn abort(&self, upload: &MultipartUpload) -> Result<(), UploadError> {
        // Use S3Client if available
        if let Some(client) = &self.client {
            client
                .abort_multipart_upload(&upload.key, &upload.upload_id)
                .await?;

            tracing::info!(
                upload_id = %upload.upload_id,
                "Aborted multipart upload"
            );

            // Record abort as failure metrics
            record_multipart_upload_failure(&upload.bucket);

            return Ok(());
        }

        // Legacy mode: just log
        tracing::info!(
            upload_id = %upload.upload_id,
            "Aborted multipart upload (legacy mode)"
        );

        // Record abort as failure metrics (legacy mode)
        record_multipart_upload_failure(&upload.bucket);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_create_multipart() {
        let handler = MultipartHandler::new("bucket", "us-east-1", MIN_PART_SIZE, 4);
        let upload = handler.create("bucket", "key").await.unwrap();

        assert!(!upload.upload_id.is_empty());
        assert_eq!(upload.bucket, "bucket");
        assert_eq!(upload.key, "key");
    }

    #[tokio::test]
    async fn test_upload_part() {
        let handler = MultipartHandler::new("bucket", "us-east-1", MIN_PART_SIZE, 4);
        let mut upload = handler.create("bucket", "key").await.unwrap();

        let body = Bytes::from(vec![0u8; MIN_PART_SIZE]);
        let part = handler.upload_part(&mut upload, 1, body).await.unwrap();

        assert_eq!(part.part_number, 1);
        assert!(!part.etag.is_empty());
    }

    #[tokio::test]
    async fn test_complete_empty_fails() {
        let handler = MultipartHandler::new("bucket", "us-east-1", MIN_PART_SIZE, 4);
        let upload = handler.create("bucket", "key").await.unwrap();

        let result = handler.complete(&upload).await;
        assert!(result.is_err());
    }
}
