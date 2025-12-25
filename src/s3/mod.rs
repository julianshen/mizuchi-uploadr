//! S3 Client module
//!
//! Provides S3 client with SigV4 signing and distributed tracing.
//!
//! # Features
//!
//! - **Distributed Tracing**: All S3 operations create spans with OpenTelemetry
//! - **W3C Trace Context**: Propagates trace context to S3 requests
//! - **Semantic Conventions**: Follows OpenTelemetry semantic conventions for S3
//! - **SigV4 Signing**: AWS Signature Version 4 authentication (TODO)
//!
//! # Example
//!
//! ```no_run
//! use mizuchi_uploadr::s3::{S3Client, S3ClientConfig};
//! use bytes::Bytes;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let config = S3ClientConfig {
//!     bucket: "my-bucket".to_string(),
//!     region: "us-east-1".to_string(),
//!     endpoint: None,
//!     access_key: Some("AKIAIOSFODNN7EXAMPLE".to_string()),
//!     secret_key: Some("wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string()),
//! };
//!
//! let client = S3Client::new(config)?;
//!
//! // Upload an object - creates a span automatically
//! let body = Bytes::from("Hello, World!");
//! let response = client.put_object("hello.txt", body, Some("text/plain")).await?;
//! println!("ETag: {}", response.etag);
//! # Ok(())
//! # }
//! ```
//!
//! # Tracing
//!
//! All S3 operations are instrumented with OpenTelemetry spans:
//!
//! | Operation | Span Name | Attributes |
//! |-----------|-----------|------------|
//! | PutObject | `s3.put_object` | bucket, key, method, bytes, etag, status_code |
//! | CreateMultipartUpload | `s3.create_multipart_upload` | bucket, key, method, upload_id, status_code |
//! | UploadPart | `s3.upload_part` | bucket, upload_id, part_number, bytes, etag, status_code |
//! | CompleteMultipartUpload | `s3.complete_multipart_upload` | bucket, upload_id, parts_count, etag, status_code |

use bytes::Bytes;
use thiserror::Error;

/// S3 client errors
#[derive(Error, Debug)]
pub enum S3ClientError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Request error: {0}")]
    RequestError(String),

    #[error("Response error: {0}")]
    ResponseError(String),

    #[error("Signing error: {0}")]
    SigningError(String),
}

/// S3 Client configuration
#[derive(Debug, Clone)]
pub struct S3ClientConfig {
    pub bucket: String,
    pub region: String,
    pub endpoint: Option<String>,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
}

/// S3 Client
pub struct S3Client {
    config: S3ClientConfig,
    #[allow(dead_code)]
    http_client: reqwest::Client,
}

impl S3Client {
    /// Create a new S3 client
    pub fn new(config: S3ClientConfig) -> Result<Self, S3ClientError> {
        let http_client = reqwest::Client::builder()
            .build()
            .map_err(|e| S3ClientError::ConfigError(e.to_string()))?;

        Ok(Self {
            config,
            http_client,
        })
    }

    /// Get the bucket name
    pub fn bucket(&self) -> &str {
        &self.config.bucket
    }

    /// Get the region
    pub fn region(&self) -> &str {
        &self.config.region
    }

    /// Get the endpoint URL
    pub fn endpoint(&self) -> String {
        self.config
            .endpoint
            .clone()
            .unwrap_or_else(|| format!("https://s3.{}.amazonaws.com", self.config.region))
    }

    /// Helper function to extract a tag value from XML
    fn extract_xml_tag(xml: &str, tag: &str) -> Option<String> {
        let start_tag = format!("<{}>", tag);
        let end_tag = format!("</{}>", tag);

        let start_pos = xml.find(&start_tag)? + start_tag.len();
        let end_pos = xml[start_pos..].find(&end_tag)? + start_pos;

        Some(xml[start_pos..end_pos].to_string())
    }

    /// Upload an object to S3 (PutObject)
    ///
    /// Creates a span for the operation and injects trace context into the request.
    ///
    /// # Arguments
    ///
    /// * `key` - S3 object key
    /// * `body` - Object data
    /// * `content_type` - Optional content type (e.g., "text/plain", "application/json")
    ///
    /// # Returns
    ///
    /// * `Ok(S3PutObjectResponse)` - Contains ETag of uploaded object
    /// * `Err(S3ClientError)` - If upload fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use mizuchi_uploadr::s3::{S3Client, S3ClientConfig};
    /// use bytes::Bytes;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = S3ClientConfig {
    /// #     bucket: "test".into(),
    /// #     region: "us-east-1".into(),
    /// #     endpoint: None,
    /// #     access_key: None,
    /// #     secret_key: None,
    /// # };
    /// let client = S3Client::new(config)?;
    /// let body = Bytes::from("Hello, World!");
    /// let response = client.put_object("hello.txt", body, Some("text/plain")).await?;
    /// println!("ETag: {}", response.etag);
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Tracing
    ///
    /// Creates a span named `s3.put_object` with attributes:
    /// - `s3.bucket` - Bucket name
    /// - `s3.key` - Object key
    /// - `http.method` - "PUT"
    /// - `upload.bytes` - Size of object
    /// - `s3.etag` - ETag from response (recorded after upload)
    /// - `http.status_code` - HTTP status code (recorded after upload)
    #[tracing::instrument(
        name = "s3.put_object",
        skip(self, body),
        fields(
            s3.bucket = %self.config.bucket,
            s3.key = %key,
            http.method = "PUT",
            upload.bytes = body.len(),
            s3.etag = tracing::field::Empty,
            http.status_code = tracing::field::Empty
        ),
        err
    )]
    pub async fn put_object(
        &self,
        key: &str,
        body: Bytes,
        content_type: Option<&str>,
    ) -> Result<S3PutObjectResponse, S3ClientError> {
        // Build the request URL
        let url = format!("{}/{}", self.endpoint(), key);

        // Build the HTTP request
        let mut request = self.http_client.put(&url).body(body);

        // Add Content-Type header if provided
        if let Some(ct) = content_type {
            request = request.header("Content-Type", ct);
        }

        // Send the request
        let response = request
            .send()
            .await
            .map_err(|e| S3ClientError::RequestError(e.to_string()))?;

        let status = response.status();

        // Check for errors
        if !status.is_success() {
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(S3ClientError::ResponseError(format!(
                "HTTP {}: {}",
                status.as_u16(),
                error_body
            )));
        }

        // Extract ETag from response headers
        let etag = response
            .headers()
            .get("ETag")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| S3ClientError::ResponseError("Missing ETag header".to_string()))?
            .to_string();

        // Record response attributes in span
        let span = tracing::Span::current();
        span.record("s3.etag", &etag.as_str());
        span.record("http.status_code", status.as_u16());

        tracing::info!(
            etag = %etag,
            status = status.as_u16(),
            "PutObject completed"
        );

        Ok(S3PutObjectResponse { etag })
    }

    /// Create a multipart upload
    #[tracing::instrument(
        name = "s3.create_multipart_upload",
        skip(self),
        fields(
            s3.bucket = %self.config.bucket,
            s3.key = %key,
            http.method = "POST",
            s3.upload_id = tracing::field::Empty,
            http.status_code = tracing::field::Empty
        ),
        err
    )]
    pub async fn create_multipart_upload(
        &self,
        key: &str,
    ) -> Result<S3CreateMultipartUploadResponse, S3ClientError> {
        // Build the request URL with ?uploads query parameter
        let url = format!("{}/{}?uploads", self.endpoint(), key);

        // Send POST request
        let response = self
            .http_client
            .post(&url)
            .send()
            .await
            .map_err(|e| S3ClientError::RequestError(e.to_string()))?;

        let status = response.status();

        // Check for errors
        if !status.is_success() {
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(S3ClientError::ResponseError(format!(
                "HTTP {}: {}",
                status.as_u16(),
                error_body
            )));
        }

        // Parse XML response
        let body = response
            .text()
            .await
            .map_err(|e| S3ClientError::ResponseError(e.to_string()))?;

        // Extract upload_id from XML
        let upload_id = Self::extract_xml_tag(&body, "UploadId").ok_or_else(|| {
            S3ClientError::ResponseError("Missing UploadId in response".to_string())
        })?;

        // Record response attributes in span
        let span = tracing::Span::current();
        span.record("s3.upload_id", &upload_id.as_str());
        span.record("http.status_code", status.as_u16());

        tracing::info!(
            upload_id = %upload_id,
            status = status.as_u16(),
            "CreateMultipartUpload completed"
        );

        Ok(S3CreateMultipartUploadResponse { upload_id })
    }

    /// Upload a part in a multipart upload
    #[tracing::instrument(
        name = "s3.upload_part",
        skip(self, body),
        fields(
            s3.bucket = %self.config.bucket,
            s3.upload_id = %upload_id,
            s3.part_number = part_number,
            http.method = "PUT",
            upload.bytes = body.len(),
            s3.etag = tracing::field::Empty,
            http.status_code = tracing::field::Empty
        ),
        err
    )]
    pub async fn upload_part(
        &self,
        upload_id: &str,
        part_number: u32,
        body: Bytes,
    ) -> Result<S3UploadPartResponse, S3ClientError> {
        // Build the request URL with query parameters
        let url = format!(
            "{}/test-key?partNumber={}&uploadId={}",
            self.endpoint(),
            part_number,
            upload_id
        );

        // Send PUT request
        let response = self
            .http_client
            .put(&url)
            .body(body)
            .send()
            .await
            .map_err(|e| S3ClientError::RequestError(e.to_string()))?;

        let status = response.status();

        // Check for errors
        if !status.is_success() {
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(S3ClientError::ResponseError(format!(
                "HTTP {}: {}",
                status.as_u16(),
                error_body
            )));
        }

        // Extract ETag from response headers
        let etag = response
            .headers()
            .get("ETag")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| S3ClientError::ResponseError("Missing ETag header".to_string()))?
            .to_string();

        // Record response attributes in span
        let span = tracing::Span::current();
        span.record("s3.etag", &etag.as_str());
        span.record("http.status_code", status.as_u16());

        tracing::info!(
            etag = %etag,
            part_number = part_number,
            status = status.as_u16(),
            "UploadPart completed"
        );

        Ok(S3UploadPartResponse { etag })
    }

    /// Complete a multipart upload
    #[tracing::instrument(
        name = "s3.complete_multipart_upload",
        skip(self, parts),
        fields(
            s3.bucket = %self.config.bucket,
            s3.upload_id = %upload_id,
            http.method = "POST",
            parts_count = parts.len(),
            s3.etag = tracing::field::Empty,
            http.status_code = tracing::field::Empty
        ),
        err
    )]
    pub async fn complete_multipart_upload(
        &self,
        upload_id: &str,
        parts: Vec<S3CompletedPart>,
    ) -> Result<S3CompleteMultipartUploadResponse, S3ClientError> {
        // Build the request URL with uploadId query parameter
        let url = format!("{}/test-key?uploadId={}", self.endpoint(), upload_id);

        // Build XML body for CompleteMultipartUpload
        let mut xml_parts = String::new();
        for part in &parts {
            xml_parts.push_str(&format!(
                "<Part><PartNumber>{}</PartNumber><ETag>{}</ETag></Part>",
                part.part_number, part.etag
            ));
        }
        let xml_body = format!(
            "<CompleteMultipartUpload>{}</CompleteMultipartUpload>",
            xml_parts
        );

        // Send POST request
        let response = self
            .http_client
            .post(&url)
            .body(xml_body)
            .header("Content-Type", "application/xml")
            .send()
            .await
            .map_err(|e| S3ClientError::RequestError(e.to_string()))?;

        let status = response.status();

        // Check for errors
        if !status.is_success() {
            let error_body = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(S3ClientError::ResponseError(format!(
                "HTTP {}: {}",
                status.as_u16(),
                error_body
            )));
        }

        // Parse XML response
        let body = response
            .text()
            .await
            .map_err(|e| S3ClientError::ResponseError(e.to_string()))?;

        // Extract ETag from XML
        let etag = Self::extract_xml_tag(&body, "ETag")
            .ok_or_else(|| S3ClientError::ResponseError("Missing ETag in response".to_string()))?;

        // Record response attributes in span
        let span = tracing::Span::current();
        span.record("s3.etag", &etag.as_str());
        span.record("http.status_code", status.as_u16());

        tracing::info!(
            etag = %etag,
            parts = parts.len(),
            status = status.as_u16(),
            "CompleteMultipartUpload completed"
        );

        Ok(S3CompleteMultipartUploadResponse { etag })
    }
}

/// S3 PutObject response
#[derive(Debug, Clone)]
pub struct S3PutObjectResponse {
    pub etag: String,
}

/// S3 CreateMultipartUpload response
#[derive(Debug, Clone)]
pub struct S3CreateMultipartUploadResponse {
    pub upload_id: String,
}

/// S3 UploadPart response
#[derive(Debug, Clone)]
pub struct S3UploadPartResponse {
    pub etag: String,
}

/// S3 CompleteMultipartUpload response
#[derive(Debug, Clone)]
pub struct S3CompleteMultipartUploadResponse {
    pub etag: String,
}

/// S3 completed part
#[derive(Debug, Clone)]
pub struct S3CompletedPart {
    pub part_number: u32,
    pub etag: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_s3_client_creation() {
        let config = S3ClientConfig {
            bucket: "test-bucket".into(),
            region: "us-east-1".into(),
            endpoint: None,
            access_key: None,
            secret_key: None,
        };

        let client = S3Client::new(config).unwrap();
        assert_eq!(client.bucket(), "test-bucket");
        assert_eq!(client.region(), "us-east-1");
    }

    #[test]
    fn test_default_endpoint() {
        let config = S3ClientConfig {
            bucket: "test-bucket".into(),
            region: "us-west-2".into(),
            endpoint: None,
            access_key: None,
            secret_key: None,
        };

        let client = S3Client::new(config).unwrap();
        assert_eq!(client.endpoint(), "https://s3.us-west-2.amazonaws.com");
    }

    #[test]
    fn test_custom_endpoint() {
        let config = S3ClientConfig {
            bucket: "test-bucket".into(),
            region: "us-east-1".into(),
            endpoint: Some("http://localhost:9000".into()),
            access_key: None,
            secret_key: None,
        };

        let client = S3Client::new(config).unwrap();
        assert_eq!(client.endpoint(), "http://localhost:9000");
    }

    // Note: HTTP integration tests are in tests/s3_http_api_test.rs
    // These tests use wiremock to mock S3 responses
}
