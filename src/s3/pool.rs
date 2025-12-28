//! S3 Client Pool Module
//!
//! Provides connection pooling for S3 clients with SigV4 signing.
//!
//! # Design
//!
//! - One AWS SDK client per configured bucket
//! - Clients are created at pool initialization
//! - Thread-safe access via HashMap with cloned Arc references
//!
//! # Example
//!
//! ```no_run
//! use mizuchi_uploadr::config::Config;
//! use mizuchi_uploadr::s3::S3ClientPool;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Load config from file
//! let config = Config::load("config.yaml")?;
//!
//! // Create pool
//! let pool = S3ClientPool::new(&config).await?;
//!
//! // Get client for a bucket
//! if let Some(client) = pool.get_client("uploads") {
//!     // Use client...
//! }
//! # Ok(())
//! # }
//! ```

use crate::config::Config;
use crate::s3::credentials::{CredentialsError, CredentialsProvider};
use crate::s3::{S3Client, S3ClientConfig, S3ClientError};
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;

/// S3 Client Pool errors
#[derive(Error, Debug)]
pub enum S3ClientPoolError {
    #[error("Credentials error: {0}")]
    CredentialsError(#[from] CredentialsError),

    #[error("Client creation error: {0}")]
    ClientCreationError(#[from] S3ClientError),
}

/// S3 Client Pool
///
/// Manages a pool of S3 clients, one per configured bucket.
/// Clients are created during pool initialization and reused for all requests.
///
/// # Thread Safety
///
/// The pool is thread-safe. Clients are wrapped in `Arc` for shared access.
pub struct S3ClientPool {
    /// Map from bucket name to S3 client
    clients: HashMap<String, Arc<S3Client>>,
}

impl S3ClientPool {
    /// Create a new S3 client pool from configuration
    ///
    /// Creates one S3 client per configured bucket. Each client is configured
    /// with the appropriate credentials, region, and endpoint.
    ///
    /// # Arguments
    ///
    /// * `config` - Application configuration containing bucket definitions
    ///
    /// # Returns
    ///
    /// * `Ok(S3ClientPool)` - Pool with initialized clients
    /// * `Err(S3ClientPoolError)` - If client creation fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use mizuchi_uploadr::config::Config;
    /// use mizuchi_uploadr::s3::S3ClientPool;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let config = Config::load("config.yaml")?;
    /// let pool = S3ClientPool::new(&config).await?;
    /// assert!(pool.client_count() >= 0);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn new(config: &Config) -> Result<Self, S3ClientPoolError> {
        let mut clients = HashMap::new();

        for bucket_config in &config.buckets {
            // Load credentials from config
            let credentials = CredentialsProvider::from_config(&bucket_config.s3)?;

            // Create S3 client config
            let client_config = S3ClientConfig {
                bucket: bucket_config.s3.bucket.clone(),
                region: bucket_config.s3.region.clone(),
                endpoint: bucket_config.s3.endpoint.clone(),
                access_key: Some(credentials.access_key_id().to_string()),
                secret_key: Some(credentials.secret_access_key().to_string()),
                retry: None,   // Use defaults
                timeout: None, // Use defaults
            };

            // Create client
            let client = S3Client::new(client_config)?;

            // Store in pool
            clients.insert(bucket_config.name.clone(), Arc::new(client));
        }

        Ok(Self { clients })
    }

    /// Get a client for a specific bucket
    ///
    /// Returns a reference to the client for the given bucket name.
    /// Returns `None` if no client exists for that bucket.
    ///
    /// # Arguments
    ///
    /// * `bucket_name` - The logical bucket name (from configuration)
    ///
    /// # Returns
    ///
    /// * `Some(&S3Client)` - Reference to the client
    /// * `None` - If no client exists for this bucket
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use mizuchi_uploadr::config::Config;
    /// # use mizuchi_uploadr::s3::S3ClientPool;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// # let config = Config::load("config.yaml")?;
    /// let pool = S3ClientPool::new(&config).await?;
    ///
    /// if let Some(client) = pool.get_client("uploads") {
    ///     println!("Got client for bucket: {}", client.bucket());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub fn get_client(&self, bucket_name: &str) -> Option<&S3Client> {
        self.clients.get(bucket_name).map(|arc| arc.as_ref())
    }

    /// Get the number of clients in the pool
    ///
    /// # Returns
    ///
    /// The number of configured S3 clients
    pub fn client_count(&self) -> usize {
        self.clients.len()
    }

    /// Get all bucket names in the pool
    ///
    /// # Returns
    ///
    /// Iterator over bucket names
    pub fn bucket_names(&self) -> impl Iterator<Item = &String> {
        self.clients.keys()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        AuthConfig, BucketConfig, MetricsConfig, S3Config, ServerConfig, UploadConfig,
        ZeroCopyConfig,
    };

    fn create_test_config(buckets: Vec<BucketConfig>) -> Config {
        Config {
            server: ServerConfig {
                address: "127.0.0.1:8080".to_string(),
                zero_copy: ZeroCopyConfig::default(),
            },
            buckets,
            metrics: MetricsConfig::default(),
            tracing: None,
        }
    }

    fn create_bucket_config(name: &str, s3_bucket: &str, region: &str) -> BucketConfig {
        BucketConfig {
            name: name.to_string(),
            path_prefix: format!("/{}", name),
            s3: S3Config {
                bucket: s3_bucket.to_string(),
                region: region.to_string(),
                endpoint: Some("http://localhost:9000".to_string()),
                access_key: Some("test-access".to_string()),
                secret_key: Some("test-secret".to_string()),
            },
            auth: AuthConfig::default(),
            upload: UploadConfig::default(),
        }
    }

    #[tokio::test]
    async fn test_pool_creation_empty() {
        let config = create_test_config(vec![]);
        let pool = S3ClientPool::new(&config).await.unwrap();
        assert_eq!(pool.client_count(), 0);
    }

    #[tokio::test]
    async fn test_pool_creation_single_bucket() {
        let config = create_test_config(vec![create_bucket_config(
            "uploads",
            "my-bucket",
            "us-east-1",
        )]);
        let pool = S3ClientPool::new(&config).await.unwrap();
        assert_eq!(pool.client_count(), 1);
    }

    #[tokio::test]
    async fn test_pool_creation_multiple_buckets() {
        let config = create_test_config(vec![
            create_bucket_config("uploads", "uploads-bucket", "us-east-1"),
            create_bucket_config("attachments", "attachments-bucket", "us-west-2"),
        ]);
        let pool = S3ClientPool::new(&config).await.unwrap();
        assert_eq!(pool.client_count(), 2);
    }

    #[tokio::test]
    async fn test_get_client_exists() {
        let config = create_test_config(vec![create_bucket_config(
            "uploads",
            "my-bucket",
            "us-east-1",
        )]);
        let pool = S3ClientPool::new(&config).await.unwrap();

        let client = pool.get_client("uploads");
        assert!(client.is_some());
        assert_eq!(client.unwrap().bucket(), "my-bucket");
    }

    #[tokio::test]
    async fn test_get_client_not_exists() {
        let config = create_test_config(vec![create_bucket_config(
            "uploads",
            "my-bucket",
            "us-east-1",
        )]);
        let pool = S3ClientPool::new(&config).await.unwrap();

        let client = pool.get_client("nonexistent");
        assert!(client.is_none());
    }

    #[tokio::test]
    async fn test_bucket_names() {
        let config = create_test_config(vec![
            create_bucket_config("uploads", "uploads-bucket", "us-east-1"),
            create_bucket_config("attachments", "attachments-bucket", "us-west-2"),
        ]);
        let pool = S3ClientPool::new(&config).await.unwrap();

        let names: Vec<_> = pool.bucket_names().collect();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&&"uploads".to_string()));
        assert!(names.contains(&&"attachments".to_string()));
    }
}
