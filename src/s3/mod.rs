//! S3 Client module
//!
//! Provides S3 client with HTTP API calls and distributed tracing.
//!
//! # Features
//!
//! - **HTTP API**: Direct HTTP requests to S3-compatible endpoints
//! - **Distributed Tracing**: All S3 operations create spans with OpenTelemetry
//! - **W3C Trace Context**: Automatic traceparent header injection for distributed tracing
//! - **XML Parsing**: Parses S3 XML responses for multipart uploads
//! - **Error Handling**: Comprehensive HTTP error handling with S3 error messages
//! - **SigV4 Signing**: AWS Signature Version 4 authentication
//! - **Connection Pool**: S3ClientPool for managing multiple bucket clients
//! - **Credentials**: Flexible credential loading from environment or config
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
//!     endpoint: Some("http://localhost:9000".to_string()), // MinIO
//!     access_key: Some("minioadmin".to_string()),
//!     secret_key: Some("minioadmin".to_string()),
//!     retry: None,   // Use default retry config (3 retries with exponential backoff)
//!     timeout: None, // Use default timeouts (5s connect, 30s request)
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
//! # Multipart Upload Example
//!
//! ```no_run
//! use mizuchi_uploadr::s3::{S3Client, S3ClientConfig, S3CompletedPart};
//! use bytes::Bytes;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! # let config = S3ClientConfig {
//! #     bucket: "my-bucket".to_string(),
//! #     region: "us-east-1".to_string(),
//! #     endpoint: Some("http://localhost:9000".to_string()),
//! #     access_key: Some("minioadmin".to_string()),
//! #     secret_key: Some("minioadmin".to_string()),
//! #     retry: None,
//! #     timeout: None,
//! # };
//! let client = S3Client::new(config)?;
//!
//! // 1. Create multipart upload
//! let key = "large-file.bin";
//! let create_response = client.create_multipart_upload(key).await?;
//! let upload_id = create_response.upload_id;
//!
//! // 2. Upload parts
//! let part1 = Bytes::from(vec![0u8; 5 * 1024 * 1024]); // 5MB
//! let part1_response = client.upload_part(key, &upload_id, 1, part1).await?;
//!
//! let part2 = Bytes::from(vec![1u8; 5 * 1024 * 1024]); // 5MB
//! let part2_response = client.upload_part(key, &upload_id, 2, part2).await?;
//!
//! // 3. Complete multipart upload
//! let parts = vec![
//!     S3CompletedPart { part_number: 1, etag: part1_response.etag },
//!     S3CompletedPart { part_number: 2, etag: part2_response.etag },
//! ];
//! let complete_response = client.complete_multipart_upload(key, &upload_id, parts).await?;
//! println!("Upload complete! ETag: {}", complete_response.etag);
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
//!
//! ## W3C Trace Context Propagation
//!
//! All S3 HTTP requests automatically include the `traceparent` header for distributed tracing:
//!
//! ```text
//! traceparent: 00-{trace-id}-{span-id}-{flags}
//! Example: 00-0000000000000000000001234567890a-0000000012345678-01
//! ```
//!
//! This enables end-to-end tracing across service boundaries, allowing you to track
//! requests from the upload proxy through to S3 and back.
//!
//! # Implementation Notes
//!
//! - **No SigV4 signing yet**: Currently sends unsigned requests (works with MinIO in dev mode)
//! - **W3C Trace Context**: Automatic traceparent injection (TODO: extract from OpenTelemetry span)
//! - **Simple XML parsing**: Uses basic string matching - consider using quick-xml for complex responses
//! - **Key parameter**: All multipart operations now accept key parameter for flexible object naming

// Sub-modules
pub mod credentials;
pub mod pool;

// Re-exports for convenience
pub use credentials::{
    Credentials, CredentialsError, CredentialsProvider, CredentialsProviderTrait,
    EnvironmentCredentials, StaticCredentials,
};
pub use pool::{S3ClientPool, S3ClientPoolError};

use aws_sigv4::http_request::{
    sign, SignableBody, SignableRequest, SigningParams, SigningSettings,
};
use aws_sigv4::sign::v4;
use bytes::Bytes;
use std::time::SystemTime;
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

/// Retry configuration for S3 operations
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// Maximum number of retry attempts (default: 3)
    pub max_retries: u32,
    /// Initial backoff delay in milliseconds (default: 100ms)
    pub initial_backoff_ms: u64,
    /// Maximum backoff delay in milliseconds (default: 10000ms)
    pub max_backoff_ms: u64,
    /// Backoff multiplier (default: 2.0)
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_backoff_ms: 100,
            max_backoff_ms: 10_000,
            backoff_multiplier: 2.0,
        }
    }
}

/// Timeout configuration for S3 operations
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    /// Connection timeout in milliseconds (default: 5000ms)
    pub connect_timeout_ms: u64,
    /// Request timeout in milliseconds (default: 30000ms)
    pub request_timeout_ms: u64,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            connect_timeout_ms: 5_000,
            request_timeout_ms: 30_000,
        }
    }
}

/// S3 Client configuration
#[derive(Debug, Clone)]
pub struct S3ClientConfig {
    pub bucket: String,
    pub region: String,
    pub endpoint: Option<String>,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
    /// Retry configuration (optional, uses defaults if not specified)
    pub retry: Option<RetryConfig>,
    /// Timeout configuration (optional, uses defaults if not specified)
    pub timeout: Option<TimeoutConfig>,
}

/// S3 Client
pub struct S3Client {
    config: S3ClientConfig,
    http_client: reqwest::Client,
    retry_config: RetryConfig,
}

impl S3Client {
    /// Create a new S3 client
    pub fn new(config: S3ClientConfig) -> Result<Self, S3ClientError> {
        let timeout_config = config.timeout.clone().unwrap_or_default();
        let retry_config = config.retry.clone().unwrap_or_default();

        let http_client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_millis(
                timeout_config.connect_timeout_ms,
            ))
            .timeout(std::time::Duration::from_millis(
                timeout_config.request_timeout_ms,
            ))
            .build()
            .map_err(|e| S3ClientError::ConfigError(e.to_string()))?;

        Ok(Self {
            config,
            http_client,
            retry_config,
        })
    }

    /// Check if an error is retryable
    fn is_retryable_error(status: reqwest::StatusCode) -> bool {
        // Retry on server errors and throttling
        status.is_server_error() // 5xx
            || status == reqwest::StatusCode::TOO_MANY_REQUESTS // 429
            || status == reqwest::StatusCode::REQUEST_TIMEOUT // 408
    }

    /// Calculate backoff delay for a retry attempt
    fn calculate_backoff(&self, attempt: u32) -> std::time::Duration {
        let delay_ms = (self.retry_config.initial_backoff_ms as f64
            * self.retry_config.backoff_multiplier.powi(attempt as i32))
        .min(self.retry_config.max_backoff_ms as f64) as u64;

        std::time::Duration::from_millis(delay_ms)
    }

    /// Compute SHA256 hash of body for x-amz-content-sha256 header
    fn compute_content_hash(body: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(body);
        hex::encode(hasher.finalize())
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

    /// Get the host from the endpoint URL
    fn get_host(&self) -> String {
        let endpoint = self.endpoint();
        // Parse the URL to extract the host
        if let Some(stripped) = endpoint.strip_prefix("https://") {
            stripped.split('/').next().unwrap_or(&endpoint).to_string()
        } else if let Some(stripped) = endpoint.strip_prefix("http://") {
            stripped.split('/').next().unwrap_or(&endpoint).to_string()
        } else {
            endpoint
        }
    }

    /// Helper function to extract a tag value from XML
    fn extract_xml_tag(xml: &str, tag: &str) -> Option<String> {
        let start_tag = format!("<{}>", tag);
        let end_tag = format!("</{}>", tag);

        let start_pos = xml.find(&start_tag)? + start_tag.len();
        let end_pos = xml[start_pos..].find(&end_tag)? + start_pos;

        Some(xml[start_pos..end_pos].to_string())
    }

    /// Inject W3C Trace Context into HTTP request
    ///
    /// Generates a traceparent header from a generated trace context.
    /// Format: 00-{trace-id}-{span-id}-{flags}
    ///
    /// For now, we generate a simple trace context using timestamp-based IDs.
    /// In the future, this should extract the actual trace context from the
    /// current OpenTelemetry span.
    fn inject_trace_context(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        // Generate a trace context using timestamp
        // TODO: Extract from current OpenTelemetry span context
        use std::time::UNIX_EPOCH;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();

        // Generate trace_id (32 hex chars) and span_id (16 hex chars) from timestamp
        let trace_id = format!("{:032x}", now);
        let span_id = format!("{:016x}", now as u64);
        let trace_flags = 0x01; // Sampled

        let traceparent = format!("00-{}-{}-{:02x}", trace_id, span_id, trace_flags);

        request.header("traceparent", traceparent)
    }

    /// Sign a request with AWS SigV4
    ///
    /// Returns the signed headers (Authorization and x-amz-date) that should be
    /// added to the HTTP request.
    ///
    /// # Arguments
    ///
    /// * `method` - HTTP method (GET, PUT, POST, DELETE)
    /// * `uri` - Full request URI including query string
    /// * `headers` - Request headers
    /// * `body` - Request body bytes
    ///
    /// # Returns
    ///
    /// * `Ok(Vec<(String, String)>)` - Signed headers to add
    /// * `Err(S3ClientError)` - If signing fails
    fn sign_request(
        &self,
        method: &str,
        uri: &str,
        headers: &[(String, String)],
        body: &[u8],
    ) -> Result<Vec<(String, String)>, S3ClientError> {
        // Get credentials from config
        let access_key = self
            .config
            .access_key
            .as_ref()
            .ok_or_else(|| S3ClientError::SigningError("Missing access key".into()))?;
        let secret_key = self
            .config
            .secret_key
            .as_ref()
            .ok_or_else(|| S3ClientError::SigningError("Missing secret key".into()))?;

        // Create credentials
        let credentials = aws_credential_types::Credentials::new(
            access_key,
            secret_key,
            None, // session token
            None, // expiration
            "mizuchi-uploadr",
        );

        // Convert to Identity for signing
        let identity = aws_smithy_runtime_api::client::identity::Identity::new(
            credentials,
            None, // expiration
        );

        // Signing settings
        let settings = SigningSettings::default();

        // Create signing params
        let signing_params = v4::SigningParams::builder()
            .identity(&identity)
            .region(&self.config.region)
            .name("s3")
            .time(SystemTime::now())
            .settings(settings)
            .build()
            .map_err(|e| S3ClientError::SigningError(e.to_string()))?;

        let signing_params = SigningParams::V4(signing_params);

        // Create signable request
        let signable_body = SignableBody::Bytes(body);
        let signable_request = SignableRequest::new(
            method,
            uri,
            headers.iter().map(|(k, v)| (k.as_str(), v.as_str())),
            signable_body,
        )
        .map_err(|e| S3ClientError::SigningError(e.to_string()))?;

        // Sign the request
        let (signing_instructions, _signature) = sign(signable_request, &signing_params)
            .map_err(|e| S3ClientError::SigningError(e.to_string()))?
            .into_parts();

        // Extract the signed headers
        let mut signed_headers = Vec::new();
        for (name, value) in signing_instructions.headers() {
            signed_headers.push((name.to_string(), value.to_string()));
        }

        Ok(signed_headers)
    }

    /// Check if this client has credentials configured for signing
    pub fn has_credentials(&self) -> bool {
        self.config.access_key.is_some() && self.config.secret_key.is_some()
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
    /// #     retry: None,
    /// #     timeout: None,
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
        // Build the request URL (path-style: /bucket/key)
        let url = format!("{}/{}/{}", self.endpoint(), self.config.bucket, key);

        // Compute content hash for x-amz-content-sha256
        let content_hash = Self::compute_content_hash(&body);

        // Build headers list for signing (including content hash)
        let mut headers = vec![
            ("host".to_string(), self.get_host()),
            ("x-amz-content-sha256".to_string(), content_hash.clone()),
        ];
        if let Some(ct) = content_type {
            headers.push(("content-type".to_string(), ct.to_string()));
        }

        // Sign the request if credentials are available
        let signed_headers = if self.has_credentials() {
            self.sign_request("PUT", &url, &headers, &body)?
        } else {
            vec![]
        };

        // Retry loop with exponential backoff
        let mut last_error = None;
        for attempt in 0..=self.retry_config.max_retries {
            if attempt > 0 {
                // Backoff before retry
                let backoff = self.calculate_backoff(attempt - 1);
                tracing::debug!(
                    attempt = attempt,
                    backoff_ms = backoff.as_millis(),
                    "Retrying S3 PutObject after backoff"
                );
                tokio::time::sleep(backoff).await;
            }

            // Build the HTTP request (need to rebuild each time for retry)
            let mut request = self.http_client.put(&url).body(body.clone());

            // Add Content-Type header if provided
            if let Some(ct) = content_type {
                request = request.header("Content-Type", ct);
            }

            // Add x-amz-content-sha256 header
            request = request.header("x-amz-content-sha256", &content_hash);

            // Add signed headers (Authorization, x-amz-date, etc.)
            for (name, value) in &signed_headers {
                request = request.header(name, value);
            }

            // Inject W3C Trace Context
            request = self.inject_trace_context(request);

            // Send the request
            let result = request.send().await;

            match result {
                Ok(response) => {
                    let status = response.status();

                    if status.is_success() {
                        // Success - extract ETag and return
                        let etag = response
                            .headers()
                            .get("ETag")
                            .and_then(|v| v.to_str().ok())
                            .ok_or_else(|| {
                                S3ClientError::ResponseError("Missing ETag header".to_string())
                            })?
                            .to_string();

                        // Record response attributes in span
                        let span = tracing::Span::current();
                        span.record("s3.etag", etag.as_str());
                        span.record("http.status_code", status.as_u16());

                        tracing::info!(
                            etag = %etag,
                            status = status.as_u16(),
                            attempts = attempt + 1,
                            "PutObject completed"
                        );

                        return Ok(S3PutObjectResponse { etag });
                    }

                    // Check if error is retryable
                    if Self::is_retryable_error(status) && attempt < self.retry_config.max_retries {
                        let error_body = response
                            .text()
                            .await
                            .unwrap_or_else(|_| "Unknown error".to_string());
                        tracing::warn!(
                            status = status.as_u16(),
                            attempt = attempt + 1,
                            error = %error_body,
                            "Retryable S3 error, will retry"
                        );
                        last_error = Some(S3ClientError::ResponseError(format!(
                            "HTTP {}: {}",
                            status.as_u16(),
                            error_body
                        )));
                        continue;
                    }

                    // Non-retryable error
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
                Err(e) => {
                    // Network error - these are typically retryable
                    if attempt < self.retry_config.max_retries {
                        tracing::warn!(
                            attempt = attempt + 1,
                            error = %e,
                            "Network error, will retry"
                        );
                        last_error = Some(S3ClientError::RequestError(e.to_string()));
                        continue;
                    }
                    return Err(S3ClientError::RequestError(e.to_string()));
                }
            }
        }

        // If we get here, all retries failed
        Err(last_error
            .unwrap_or_else(|| S3ClientError::RequestError("All retries exhausted".to_string())))
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
        // Build the request URL with ?uploads query parameter (path-style: /bucket/key?uploads)
        let url = format!("{}/{}/{}?uploads", self.endpoint(), self.config.bucket, key);

        // Build POST request with trace context
        let request = self.http_client.post(&url);
        let request = self.inject_trace_context(request);

        // Send POST request
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
        span.record("s3.upload_id", upload_id.as_str());
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
            s3.key = %key,
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
        key: &str,
        upload_id: &str,
        part_number: u32,
        body: Bytes,
    ) -> Result<S3UploadPartResponse, S3ClientError> {
        // Build the request URL with query parameters (path-style: /bucket/key?...)
        let url = format!(
            "{}/{}/{}?partNumber={}&uploadId={}",
            self.endpoint(),
            self.config.bucket,
            key,
            part_number,
            upload_id
        );

        // Build PUT request with trace context
        let request = self.http_client.put(&url).body(body);
        let request = self.inject_trace_context(request);

        // Send PUT request
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
        span.record("s3.etag", etag.as_str());
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
            s3.key = %key,
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
        key: &str,
        upload_id: &str,
        parts: Vec<S3CompletedPart>,
    ) -> Result<S3CompleteMultipartUploadResponse, S3ClientError> {
        // Build the request URL with uploadId query parameter (path-style: /bucket/key?...)
        let url = format!(
            "{}/{}/{}?uploadId={}",
            self.endpoint(),
            self.config.bucket,
            key,
            upload_id
        );

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

        // Build POST request with trace context
        let request = self
            .http_client
            .post(&url)
            .body(xml_body)
            .header("Content-Type", "application/xml");
        let request = self.inject_trace_context(request);

        // Send POST request
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
        span.record("s3.etag", etag.as_str());
        span.record("http.status_code", status.as_u16());

        tracing::info!(
            etag = %etag,
            parts = parts.len(),
            status = status.as_u16(),
            "CompleteMultipartUpload completed"
        );

        Ok(S3CompleteMultipartUploadResponse { etag })
    }

    /// Abort a multipart upload
    #[tracing::instrument(
        name = "s3.abort_multipart_upload",
        skip(self),
        fields(
            s3.bucket = %self.config.bucket,
            s3.key = %key,
            s3.upload_id = %upload_id,
            http.method = "DELETE",
            http.status_code = tracing::field::Empty
        ),
        err
    )]
    pub async fn abort_multipart_upload(
        &self,
        key: &str,
        upload_id: &str,
    ) -> Result<(), S3ClientError> {
        // Build the request URL with uploadId query parameter (path-style: /bucket/key?...)
        let url = format!(
            "{}/{}/{}?uploadId={}",
            self.endpoint(),
            self.config.bucket,
            key,
            upload_id
        );

        // Build DELETE request with trace context
        let request = self.http_client.delete(&url);
        let request = self.inject_trace_context(request);

        // Send DELETE request
        let response = request
            .send()
            .await
            .map_err(|e| S3ClientError::RequestError(e.to_string()))?;

        let status = response.status();

        // Check for errors (204 No Content is success for abort)
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

        // Record response attributes in span
        let span = tracing::Span::current();
        span.record("http.status_code", status.as_u16());

        tracing::info!(
            upload_id = %upload_id,
            status = status.as_u16(),
            "AbortMultipartUpload completed"
        );

        Ok(())
    }

    /// Upload an object from a temp file (zero-copy optimized)
    ///
    /// Uses the pre-computed content hash from TempFileUpload for SigV4 signing,
    /// avoiding the need to re-hash the body. On Linux with tmpfs, this provides
    /// near-zero-copy performance since the file is already in RAM.
    ///
    /// # Arguments
    ///
    /// * `key` - S3 object key
    /// * `temp_file` - TempFileUpload with pre-computed hash
    /// * `content_type` - Optional content type
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
    /// use mizuchi_uploadr::upload::temp_file::TempFileUpload;
    /// use bytes::Bytes;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = S3ClientConfig {
    /// #     bucket: "test".into(),
    /// #     region: "us-east-1".into(),
    /// #     endpoint: None,
    /// #     access_key: None,
    /// #     secret_key: None,
    /// #     retry: None,
    /// #     timeout: None,
    /// # };
    /// let client = S3Client::new(config)?;
    ///
    /// // Create temp file with pre-computed hash
    /// let data = Bytes::from(vec![0u8; 10 * 1024 * 1024]); // 10MB
    /// let temp = TempFileUpload::from_bytes(data)?;
    ///
    /// // Upload using pre-computed hash (no re-hashing needed)
    /// let response = client.put_object_from_file("large-file.bin", &temp, None).await?;
    /// println!("ETag: {}", response.etag);
    /// # Ok(())
    /// # }
    /// ```
    #[tracing::instrument(
        name = "s3.put_object_from_file",
        skip(self, temp_file),
        fields(
            s3.bucket = %self.config.bucket,
            s3.key = %key,
            http.method = "PUT",
            upload.bytes = temp_file.size(),
            upload.mode = "temp_file",
            s3.etag = tracing::field::Empty,
            http.status_code = tracing::field::Empty
        ),
        err
    )]
    pub async fn put_object_from_file(
        &self,
        key: &str,
        temp_file: &crate::upload::temp_file::TempFileUpload,
        content_type: Option<&str>,
    ) -> Result<S3PutObjectResponse, S3ClientError> {
        use std::io::Read;

        // Read file content into memory
        // Note: For large files, this could be optimized with streaming
        // In REFACTOR phase, we can use reqwest's Body::wrap_stream
        let mut file = std::fs::File::open(temp_file.path())
            .map_err(|e| S3ClientError::RequestError(format!("Failed to open temp file: {}", e)))?;

        let mut body = Vec::with_capacity(temp_file.size() as usize);
        file.read_to_end(&mut body)
            .map_err(|e| S3ClientError::RequestError(format!("Failed to read temp file: {}", e)))?;

        let body = Bytes::from(body);

        // Use pre-computed content hash (avoids re-hashing)
        let content_hash = temp_file.content_hash().to_string();

        // Record zero-copy mode in metrics
        crate::metrics::record_data_transfer(temp_file.size(), 0.0, temp_file.supports_zero_copy());

        // Build the request URL (path-style: /bucket/key)
        let url = format!("{}/{}/{}", self.endpoint(), self.config.bucket, key);

        // Build headers list for signing (including pre-computed content hash)
        let mut headers = vec![
            ("host".to_string(), self.get_host()),
            ("x-amz-content-sha256".to_string(), content_hash.clone()),
        ];
        if let Some(ct) = content_type {
            headers.push(("content-type".to_string(), ct.to_string()));
        }

        // Sign the request if credentials are available
        let signed_headers = if self.has_credentials() {
            self.sign_request("PUT", &url, &headers, &body)?
        } else {
            vec![]
        };

        // Retry loop with exponential backoff
        let mut last_error = None;
        for attempt in 0..=self.retry_config.max_retries {
            if attempt > 0 {
                let backoff = self.calculate_backoff(attempt - 1);
                tracing::debug!(
                    attempt = attempt,
                    backoff_ms = backoff.as_millis(),
                    "Retrying S3 PutObject from file after backoff"
                );
                tokio::time::sleep(backoff).await;
            }

            // Build the HTTP request
            let mut request = self.http_client.put(&url).body(body.clone());

            // Add Content-Type header if provided
            if let Some(ct) = content_type {
                request = request.header("Content-Type", ct);
            }

            // Add x-amz-content-sha256 header (pre-computed)
            request = request.header("x-amz-content-sha256", &content_hash);

            // Add signed headers
            for (name, value) in &signed_headers {
                request = request.header(name, value);
            }

            // Inject W3C Trace Context
            request = self.inject_trace_context(request);

            // Send the request
            let result = request.send().await;

            match result {
                Ok(response) => {
                    let status = response.status();

                    if status.is_success() {
                        let etag = response
                            .headers()
                            .get("ETag")
                            .and_then(|v| v.to_str().ok())
                            .ok_or_else(|| {
                                S3ClientError::ResponseError("Missing ETag header".to_string())
                            })?
                            .to_string();

                        // Record response attributes in span
                        let span = tracing::Span::current();
                        span.record("s3.etag", etag.as_str());
                        span.record("http.status_code", status.as_u16());

                        tracing::info!(
                            etag = %etag,
                            status = status.as_u16(),
                            attempts = attempt + 1,
                            mode = "temp_file",
                            "PutObject from file completed"
                        );

                        return Ok(S3PutObjectResponse { etag });
                    }

                    // Check if error is retryable
                    if Self::is_retryable_error(status) && attempt < self.retry_config.max_retries {
                        let error_body = response
                            .text()
                            .await
                            .unwrap_or_else(|_| "Unknown error".to_string());
                        tracing::warn!(
                            status = status.as_u16(),
                            attempt = attempt + 1,
                            error = %error_body,
                            "Retryable S3 error, will retry"
                        );
                        last_error = Some(S3ClientError::ResponseError(format!(
                            "HTTP {}: {}",
                            status.as_u16(),
                            error_body
                        )));
                        continue;
                    }

                    // Non-retryable error
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
                Err(e) => {
                    if attempt < self.retry_config.max_retries {
                        tracing::warn!(
                            attempt = attempt + 1,
                            error = %e,
                            "Network error, will retry"
                        );
                        last_error = Some(S3ClientError::RequestError(e.to_string()));
                        continue;
                    }
                    return Err(S3ClientError::RequestError(e.to_string()));
                }
            }
        }

        Err(last_error
            .unwrap_or_else(|| S3ClientError::RequestError("All retries exhausted".to_string())))
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
            retry: None,
            timeout: None,
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
            retry: None,
            timeout: None,
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
            retry: None,
            timeout: None,
        };

        let client = S3Client::new(config).unwrap();
        assert_eq!(client.endpoint(), "http://localhost:9000");
    }

    #[test]
    fn test_retry_config_defaults() {
        let config = RetryConfig::default();
        assert_eq!(config.max_retries, 3);
        assert_eq!(config.initial_backoff_ms, 100);
        assert_eq!(config.max_backoff_ms, 10_000);
        assert!((config.backoff_multiplier - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_timeout_config_defaults() {
        let config = TimeoutConfig::default();
        assert_eq!(config.connect_timeout_ms, 5_000);
        assert_eq!(config.request_timeout_ms, 30_000);
    }

    #[test]
    fn test_custom_retry_config() {
        let config = S3ClientConfig {
            bucket: "test-bucket".into(),
            region: "us-east-1".into(),
            endpoint: None,
            access_key: None,
            secret_key: None,
            retry: Some(RetryConfig {
                max_retries: 5,
                initial_backoff_ms: 200,
                max_backoff_ms: 20_000,
                backoff_multiplier: 3.0,
            }),
            timeout: None,
        };

        let client = S3Client::new(config).unwrap();
        assert_eq!(client.retry_config.max_retries, 5);
        assert_eq!(client.retry_config.initial_backoff_ms, 200);
    }

    #[test]
    fn test_is_retryable_error() {
        use reqwest::StatusCode;

        // 5xx errors are retryable
        assert!(S3Client::is_retryable_error(
            StatusCode::INTERNAL_SERVER_ERROR
        ));
        assert!(S3Client::is_retryable_error(StatusCode::BAD_GATEWAY));
        assert!(S3Client::is_retryable_error(
            StatusCode::SERVICE_UNAVAILABLE
        ));

        // 429 Too Many Requests is retryable
        assert!(S3Client::is_retryable_error(StatusCode::TOO_MANY_REQUESTS));

        // 408 Request Timeout is retryable
        assert!(S3Client::is_retryable_error(StatusCode::REQUEST_TIMEOUT));

        // 4xx errors (except 408, 429) are not retryable
        assert!(!S3Client::is_retryable_error(StatusCode::BAD_REQUEST));
        assert!(!S3Client::is_retryable_error(StatusCode::NOT_FOUND));
        assert!(!S3Client::is_retryable_error(StatusCode::FORBIDDEN));
    }

    #[test]
    fn test_content_hash_computation() {
        // Empty body
        let hash = S3Client::compute_content_hash(b"");
        assert_eq!(
            hash,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );

        // "hello" - known SHA256 hash
        let hash = S3Client::compute_content_hash(b"hello");
        assert_eq!(
            hash,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn test_calculate_backoff() {
        let config = S3ClientConfig {
            bucket: "test".into(),
            region: "us-east-1".into(),
            endpoint: None,
            access_key: None,
            secret_key: None,
            retry: Some(RetryConfig {
                max_retries: 5,
                initial_backoff_ms: 100,
                max_backoff_ms: 10_000,
                backoff_multiplier: 2.0,
            }),
            timeout: None,
        };

        let client = S3Client::new(config).unwrap();

        // Attempt 0: 100ms * 2^0 = 100ms
        assert_eq!(
            client.calculate_backoff(0),
            std::time::Duration::from_millis(100)
        );

        // Attempt 1: 100ms * 2^1 = 200ms
        assert_eq!(
            client.calculate_backoff(1),
            std::time::Duration::from_millis(200)
        );

        // Attempt 2: 100ms * 2^2 = 400ms
        assert_eq!(
            client.calculate_backoff(2),
            std::time::Duration::from_millis(400)
        );

        // High attempt should cap at max_backoff_ms
        assert_eq!(
            client.calculate_backoff(10),
            std::time::Duration::from_millis(10_000)
        );
    }

    // Note: HTTP integration tests are in tests/s3_http_api_test.rs
    // These tests use wiremock to mock S3 responses
}
