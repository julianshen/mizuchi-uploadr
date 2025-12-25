//! Upload module
//!
//! Handles S3 upload operations with zero-copy optimization on Linux.

use thiserror::Error;

pub mod multipart;
pub mod put_object;
pub mod zero_copy;

/// Upload errors
#[derive(Error, Debug)]
pub enum UploadError {
    #[error("S3 error: {0}")]
    S3Error(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Zero-copy not available")]
    ZeroCopyUnavailable,

    #[error("Invalid content length")]
    InvalidContentLength,

    #[error("Part too small (minimum 5MB)")]
    PartTooSmall,

    #[error("Multipart upload error: {0}")]
    MultipartError(String),
}

/// Upload result
#[derive(Debug, Clone)]
pub struct UploadResult {
    pub etag: String,
    pub version_id: Option<String>,
    pub bytes_written: u64,
}

/// Upload handler trait
#[async_trait::async_trait]
pub trait UploadHandler: Send + Sync {
    /// Handle upload
    async fn upload(
        &self,
        bucket: &str,
        key: &str,
        body: bytes::Bytes,
        content_type: Option<&str>,
    ) -> Result<UploadResult, UploadError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_upload_result() {
        let result = UploadResult {
            etag: "abc123".into(),
            version_id: None,
            bytes_written: 1024,
        };
        assert_eq!(result.bytes_written, 1024);
    }
}
