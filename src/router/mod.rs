//! S3 API Router
//!
//! Parses incoming requests and routes them to appropriate handlers.
//! Provides bucket resolution to map path prefixes to S3 bucket configurations.
//!
//! # Performance
//!
//! The `BucketResolver` uses a HashMap for O(1) average-case lookup performance.
//! Path prefixes are normalized and stored as keys for fast resolution.

use crate::config::{BucketConfig, Config};
use std::collections::HashMap;
use thiserror::Error;

/// Router errors
#[derive(Error, Debug)]
pub enum RouterError {
    #[error("Invalid path: {0}")]
    InvalidPath(String),

    #[error("Method not allowed: {0}")]
    MethodNotAllowed(String),

    #[error("Bucket not found: {0}")]
    BucketNotFound(String),
}

/// S3 operation types
#[derive(Debug, Clone, PartialEq)]
pub enum S3Operation {
    /// PUT /{bucket}/{key}
    PutObject { bucket: String, key: String },
    /// POST /{bucket}/{key}?uploads
    CreateMultipartUpload { bucket: String, key: String },
    /// PUT /{bucket}/{key}?partNumber=N&uploadId=X
    UploadPart {
        bucket: String,
        key: String,
        part_number: u32,
        upload_id: String,
    },
    /// POST /{bucket}/{key}?uploadId=X
    CompleteMultipartUpload {
        bucket: String,
        key: String,
        upload_id: String,
    },
    /// DELETE /{bucket}/{key}?uploadId=X
    AbortMultipartUpload {
        bucket: String,
        key: String,
        upload_id: String,
    },
    /// GET /{bucket}/{key}?uploadId=X
    ListParts {
        bucket: String,
        key: String,
        upload_id: String,
    },
}

/// S3 Request Parser
pub struct S3RequestParser;

impl S3RequestParser {
    /// Parse an HTTP request into an S3 operation
    pub fn parse(
        method: &str,
        path: &str,
        query: Option<&str>,
    ) -> Result<S3Operation, RouterError> {
        let path = path.trim_start_matches('/');
        let parts: Vec<&str> = path.splitn(2, '/').collect();

        if parts.is_empty() || parts[0].is_empty() {
            return Err(RouterError::InvalidPath("Missing bucket".into()));
        }

        let bucket = parts[0].to_string();
        let key = parts.get(1).map(|s| s.to_string()).unwrap_or_default();

        if key.is_empty() && method != "HEAD" {
            return Err(RouterError::InvalidPath("Missing key".into()));
        }

        let query_params = Self::parse_query(query);

        match method {
            "PUT" => {
                if let (Some(part_number), Some(upload_id)) =
                    (query_params.get("partNumber"), query_params.get("uploadId"))
                {
                    Ok(S3Operation::UploadPart {
                        bucket,
                        key,
                        part_number: part_number.parse().unwrap_or(0),
                        upload_id: upload_id.clone(),
                    })
                } else {
                    Ok(S3Operation::PutObject { bucket, key })
                }
            }
            "POST" => {
                if query_params.contains_key("uploads") {
                    Ok(S3Operation::CreateMultipartUpload { bucket, key })
                } else if let Some(upload_id) = query_params.get("uploadId") {
                    Ok(S3Operation::CompleteMultipartUpload {
                        bucket,
                        key,
                        upload_id: upload_id.clone(),
                    })
                } else {
                    Err(RouterError::InvalidPath("Invalid POST operation".into()))
                }
            }
            "DELETE" => {
                if let Some(upload_id) = query_params.get("uploadId") {
                    Ok(S3Operation::AbortMultipartUpload {
                        bucket,
                        key,
                        upload_id: upload_id.clone(),
                    })
                } else {
                    Err(RouterError::MethodNotAllowed(
                        "DELETE only allowed for multipart abort".into(),
                    ))
                }
            }
            "GET" => {
                if let Some(upload_id) = query_params.get("uploadId") {
                    Ok(S3Operation::ListParts {
                        bucket,
                        key,
                        upload_id: upload_id.clone(),
                    })
                } else {
                    Err(RouterError::MethodNotAllowed(
                        "GET not allowed (upload-only proxy)".into(),
                    ))
                }
            }
            _ => Err(RouterError::MethodNotAllowed(format!(
                "Method {} not allowed",
                method
            ))),
        }
    }

    fn parse_query(query: Option<&str>) -> std::collections::HashMap<String, String> {
        let mut params = std::collections::HashMap::new();
        if let Some(q) = query {
            for pair in q.split('&') {
                let mut kv = pair.splitn(2, '=');
                if let Some(key) = kv.next() {
                    let value = kv.next().unwrap_or("");
                    params.insert(key.to_string(), value.to_string());
                }
            }
        }
        params
    }
}

/// Bucket Resolver
///
/// Maps incoming request paths to configured S3 buckets using a HashMap for O(1) lookup.
///
/// # Performance
///
/// Uses a HashMap to map path prefixes to bucket configurations, providing O(1)
/// average-case lookup performance instead of O(n) linear search.
///
/// # Example
///
/// ```
/// use mizuchi_uploadr::config::{Config, BucketConfig, S3Config, ServerConfig, ZeroCopyConfig, AuthConfig, UploadConfig, MetricsConfig};
/// use mizuchi_uploadr::router::BucketResolver;
///
/// # fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Create a test configuration
/// let config = Config {
///     server: ServerConfig {
///         address: "127.0.0.1:8080".to_string(),
///         zero_copy: ZeroCopyConfig::default(),
///     },
///     buckets: vec![
///         BucketConfig {
///             name: "uploads".to_string(),
///             path_prefix: "/uploads".to_string(),
///             s3: S3Config {
///                 bucket: "my-bucket".to_string(),
///                 region: "us-east-1".to_string(),
///                 endpoint: None,
///                 access_key: None,
///                 secret_key: None,
///             },
///             auth: AuthConfig::default(),
///             upload: UploadConfig::default(),
///         },
///     ],
///     metrics: MetricsConfig::default(),
///     tracing: None,
/// };
///
/// let resolver = BucketResolver::new(&config);
///
/// // Resolve a path to a bucket configuration
/// let bucket = resolver.resolve_bucket("/uploads/file.txt")?;
/// println!("Bucket: {}", bucket.name);
///
/// // Resolve and extract S3 key
/// let (bucket, key) = resolver.resolve_bucket_and_key("/uploads/folder/file.txt")?;
/// println!("Bucket: {}, Key: {}", bucket.name, key);
/// # Ok(())
/// # }
/// ```
pub struct BucketResolver {
    /// HashMap mapping path prefixes to bucket configurations
    /// Key: normalized path prefix (e.g., "/uploads")
    /// Value: bucket configuration
    prefix_map: HashMap<String, BucketConfig>,
}

impl BucketResolver {
    /// Create a new bucket resolver from configuration
    ///
    /// Builds a HashMap from the bucket configurations for fast O(1) lookup.
    ///
    /// # Arguments
    ///
    /// * `config` - Application configuration containing bucket definitions
    ///
    /// # Example
    ///
    /// ```
    /// # use mizuchi_uploadr::config::{Config, BucketConfig, S3Config, ServerConfig, ZeroCopyConfig, AuthConfig, UploadConfig, MetricsConfig};
    /// # use mizuchi_uploadr::router::BucketResolver;
    /// # let config = Config {
    /// #     server: ServerConfig { address: "127.0.0.1:8080".to_string(), zero_copy: ZeroCopyConfig::default() },
    /// #     buckets: vec![],
    /// #     metrics: MetricsConfig::default(),
    /// #     tracing: None,
    /// # };
    /// let resolver = BucketResolver::new(&config);
    /// ```
    pub fn new(config: &Config) -> Self {
        let mut prefix_map = HashMap::new();

        for bucket in &config.buckets {
            // Normalize prefix (ensure it starts with / and doesn't end with /)
            let normalized_prefix = Self::normalize_prefix(&bucket.path_prefix);
            prefix_map.insert(normalized_prefix, bucket.clone());
        }

        Self { prefix_map }
    }

    /// Normalize a path prefix
    ///
    /// Ensures the prefix starts with / and doesn't end with / (unless it's just "/")
    fn normalize_prefix(prefix: &str) -> String {
        let mut normalized = prefix.to_string();

        // Ensure starts with /
        if !normalized.starts_with('/') {
            normalized.insert(0, '/');
        }

        // Remove trailing / (unless it's the root)
        if normalized.len() > 1 && normalized.ends_with('/') {
            normalized.pop();
        }

        normalized
    }

    /// Extract the first path segment from a path
    ///
    /// Used to determine which bucket prefix to look up.
    fn extract_first_segment(path: &str) -> Option<String> {
        let trimmed = path.trim_start_matches('/');
        trimmed.split('/').next().map(|s| format!("/{}", s))
    }

    /// Resolve a path to a bucket configuration
    ///
    /// Looks up the bucket configuration based on the path prefix using O(1) HashMap lookup.
    ///
    /// # Arguments
    ///
    /// * `path` - The request path (e.g., "/uploads/file.txt")
    ///
    /// # Returns
    ///
    /// * `Ok(&BucketConfig)` - The matching bucket configuration
    /// * `Err(RouterError)` - If no bucket matches or path is invalid
    ///
    /// # Example
    ///
    /// ```
    /// # use mizuchi_uploadr::config::{Config, BucketConfig, S3Config, ServerConfig, ZeroCopyConfig, AuthConfig, UploadConfig, MetricsConfig};
    /// # use mizuchi_uploadr::router::BucketResolver;
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = Config {
    /// #     server: ServerConfig { address: "127.0.0.1:8080".to_string(), zero_copy: ZeroCopyConfig::default() },
    /// #     buckets: vec![
    /// #         BucketConfig {
    /// #             name: "uploads".to_string(),
    /// #             path_prefix: "/uploads".to_string(),
    /// #             s3: S3Config { bucket: "my-bucket".to_string(), region: "us-east-1".to_string(), endpoint: None, access_key: None, secret_key: None },
    /// #             auth: AuthConfig::default(),
    /// #             upload: UploadConfig::default(),
    /// #         },
    /// #     ],
    /// #     metrics: MetricsConfig::default(),
    /// #     tracing: None,
    /// # };
    /// let resolver = BucketResolver::new(&config);
    /// let bucket = resolver.resolve_bucket("/uploads/file.txt")?;
    /// assert_eq!(bucket.name, "uploads");
    /// # Ok(())
    /// # }
    /// ```
    pub fn resolve_bucket(&self, path: &str) -> Result<&BucketConfig, RouterError> {
        // Handle empty or root path
        if path.is_empty() || path == "/" {
            return Err(RouterError::InvalidPath("Empty or root path".into()));
        }

        // Normalize path (ensure it starts with /)
        if !path.starts_with('/') {
            return Err(RouterError::InvalidPath("Path must start with /".into()));
        }

        // Extract first segment and look up in HashMap
        let first_segment = Self::extract_first_segment(path)
            .ok_or_else(|| RouterError::InvalidPath("Invalid path format".into()))?;

        self.prefix_map.get(&first_segment).ok_or_else(|| {
            RouterError::BucketNotFound(format!(
                "No bucket configured for path prefix: {}",
                first_segment
            ))
        })
    }

    /// Resolve a path to a bucket configuration and extract the S3 key
    ///
    /// Combines bucket resolution with S3 key extraction in one operation.
    ///
    /// # Arguments
    ///
    /// * `path` - The request path (e.g., "/uploads/folder/file.txt")
    ///
    /// # Returns
    ///
    /// * `Ok((&BucketConfig, String))` - The bucket config and extracted S3 key
    /// * `Err(RouterError)` - If no bucket matches or path is invalid
    ///
    /// # Example
    ///
    /// ```
    /// # use mizuchi_uploadr::config::{Config, BucketConfig, S3Config, ServerConfig, ZeroCopyConfig, AuthConfig, UploadConfig, MetricsConfig};
    /// # use mizuchi_uploadr::router::BucketResolver;
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = Config {
    /// #     server: ServerConfig { address: "127.0.0.1:8080".to_string(), zero_copy: ZeroCopyConfig::default() },
    /// #     buckets: vec![
    /// #         BucketConfig {
    /// #             name: "uploads".to_string(),
    /// #             path_prefix: "/uploads".to_string(),
    /// #             s3: S3Config { bucket: "my-bucket".to_string(), region: "us-east-1".to_string(), endpoint: None, access_key: None, secret_key: None },
    /// #             auth: AuthConfig::default(),
    /// #             upload: UploadConfig::default(),
    /// #         },
    /// #     ],
    /// #     metrics: MetricsConfig::default(),
    /// #     tracing: None,
    /// # };
    /// let resolver = BucketResolver::new(&config);
    /// let (bucket, key) = resolver.resolve_bucket_and_key("/uploads/folder/file.txt")?;
    /// assert_eq!(bucket.name, "uploads");
    /// assert_eq!(key, "folder/file.txt");
    /// # Ok(())
    /// # }
    /// ```
    pub fn resolve_bucket_and_key(
        &self,
        path: &str,
    ) -> Result<(&BucketConfig, String), RouterError> {
        let bucket = self.resolve_bucket(path)?;

        // Extract the S3 key (path after bucket prefix)
        let key = path[bucket.path_prefix.len()..]
            .trim_start_matches('/')
            .to_string();

        Ok((bucket, key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_put_object() {
        let op = S3RequestParser::parse("PUT", "/bucket/key", None).unwrap();
        assert_eq!(
            op,
            S3Operation::PutObject {
                bucket: "bucket".into(),
                key: "key".into()
            }
        );
    }

    #[test]
    fn test_parse_create_multipart() {
        let op = S3RequestParser::parse("POST", "/bucket/key", Some("uploads")).unwrap();
        assert_eq!(
            op,
            S3Operation::CreateMultipartUpload {
                bucket: "bucket".into(),
                key: "key".into()
            }
        );
    }

    #[test]
    fn test_parse_upload_part() {
        let op = S3RequestParser::parse("PUT", "/bucket/key", Some("partNumber=1&uploadId=abc123"))
            .unwrap();
        assert_eq!(
            op,
            S3Operation::UploadPart {
                bucket: "bucket".into(),
                key: "key".into(),
                part_number: 1,
                upload_id: "abc123".into()
            }
        );
    }

    #[test]
    fn test_parse_get_not_allowed() {
        let result = S3RequestParser::parse("GET", "/bucket/key", None);
        assert!(result.is_err());
    }
}
