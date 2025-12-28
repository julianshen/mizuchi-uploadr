//! Common E2E Test Infrastructure
//!
//! Provides shared utilities for E2E tests:
//! - Test server management
//! - S3 client helpers
//! - JWT token generation
//! - SigV4 request signing
//! - Test data generation

use bytes::Bytes;
use mizuchi_uploadr::config::{
    AuthConfig, BucketConfig, Config, JwtConfig, MetricsConfig, S3Config, ServerConfig,
    UploadConfig, ZeroCopyConfig,
};
use mizuchi_uploadr::server::pingora::PingoraServer;
use std::net::SocketAddr;
use std::time::Duration;

/// Default RustFS endpoint for E2E tests
pub const RUSTFS_ENDPOINT: &str = "http://localhost:9000";

/// Default credentials for RustFS
pub const RUSTFS_ACCESS_KEY: &str = "minioadmin";
pub const RUSTFS_SECRET_KEY: &str = "minioadmin";

/// Test bucket name
pub const TEST_BUCKET: &str = "e2e-test-bucket";

/// JWT secret for test tokens
pub const JWT_SECRET: &str = "e2e-test-secret-key-for-jwt-tokens";

/// E2E Test Environment
///
/// Manages the test server and provides utilities for making requests.
pub struct E2ETestEnv {
    pub server_addr: SocketAddr,
    pub s3_endpoint: String,
    pub client: reqwest::Client,
    _server_handle: tokio::task::JoinHandle<()>,
}

impl E2ETestEnv {
    /// Create a new E2E test environment with a running server
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        Self::with_config(Self::default_config(0)).await
    }

    /// Create a new E2E test environment with custom config
    pub async fn with_config(config: Config) -> Result<Self, Box<dyn std::error::Error>> {
        let server = PingoraServer::new(config).await?;
        let server_addr = server.local_addr()?;

        let server_handle = tokio::spawn(async move {
            let _ = server.run().await;
        });

        // Wait for server to be ready
        tokio::time::sleep(Duration::from_millis(100)).await;

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()?;

        Ok(Self {
            server_addr,
            s3_endpoint: RUSTFS_ENDPOINT.to_string(),
            client,
            _server_handle: server_handle,
        })
    }

    /// Get the base URL for the test server
    pub fn base_url(&self) -> String {
        format!("http://{}", self.server_addr)
    }

    /// Upload a file via PUT request
    pub async fn put_object(
        &self,
        path: &str,
        body: impl AsRef<[u8]>,
        content_type: Option<&str>,
    ) -> Result<reqwest::Response, reqwest::Error> {
        let mut request = self.client.put(format!("{}{}", self.base_url(), path));

        if let Some(ct) = content_type {
            request = request.header("Content-Type", ct);
        }

        request.body(body.as_ref().to_vec()).send().await
    }

    /// Upload a file with JWT authentication
    pub async fn put_object_with_jwt(
        &self,
        path: &str,
        body: impl AsRef<[u8]>,
        token: &str,
    ) -> Result<reqwest::Response, reqwest::Error> {
        self.client
            .put(format!("{}{}", self.base_url(), path))
            .header("Authorization", format!("Bearer {}", token))
            .body(body.as_ref().to_vec())
            .send()
            .await
    }

    /// Generate a random test payload
    pub fn random_payload(size: usize) -> Bytes {
        use rand::Rng;
        let mut rng = rand::rng();
        let data: Vec<u8> = (0..size).map(|_| rng.random()).collect();
        Bytes::from(data)
    }

    /// Generate a valid JWT token for testing
    pub fn generate_test_jwt(subject: &str, expires_in_secs: u64) -> String {
        use jsonwebtoken::{encode, EncodingKey, Header};
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Serialize, Deserialize)]
        struct Claims {
            sub: String,
            exp: u64,
            iat: u64,
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let claims = Claims {
            sub: subject.to_string(),
            exp: now + expires_in_secs,
            iat: now,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
        )
        .expect("Failed to generate JWT")
    }

    /// Generate an expired JWT token for testing
    pub fn generate_expired_jwt(subject: &str) -> String {
        use jsonwebtoken::{encode, EncodingKey, Header};
        use serde::{Deserialize, Serialize};

        #[derive(Debug, Serialize, Deserialize)]
        struct Claims {
            sub: String,
            exp: u64,
            iat: u64,
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let claims = Claims {
            sub: subject.to_string(),
            exp: now - 3600, // Expired 1 hour ago
            iat: now - 7200, // Issued 2 hours ago
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(JWT_SECRET.as_bytes()),
        )
        .expect("Failed to generate JWT")
    }

    /// Default test configuration
    pub fn default_config(port: u16) -> Config {
        Config {
            server: ServerConfig {
                address: format!("127.0.0.1:{}", port),
                zero_copy: ZeroCopyConfig::default(),
            },
            buckets: vec![BucketConfig {
                name: TEST_BUCKET.into(),
                path_prefix: "/uploads".into(),
                s3: S3Config {
                    bucket: TEST_BUCKET.into(),
                    region: "us-east-1".into(),
                    endpoint: Some(RUSTFS_ENDPOINT.into()),
                    access_key: Some(RUSTFS_ACCESS_KEY.into()),
                    secret_key: Some(RUSTFS_SECRET_KEY.into()),
                },
                auth: AuthConfig::default(),
                upload: UploadConfig::default(),
            }],
            metrics: MetricsConfig::default(),
            tracing: None,
        }
    }

    /// Configuration with JWT authentication enabled
    pub fn config_with_jwt(port: u16) -> Config {
        let mut config = Self::default_config(port);
        config.buckets[0].auth = AuthConfig {
            enabled: true,
            jwt: Some(JwtConfig {
                secret: Some(JWT_SECRET.into()),
                algorithm: "HS256".into(),
                jwks_url: None,
                token_sources: vec![],
            }),
            sigv4: None,
        };
        config
    }
}

impl Drop for E2ETestEnv {
    fn drop(&mut self) {
        self._server_handle.abort();
    }
}

/// Check if RustFS/S3 backend is available
pub async fn is_s3_backend_available() -> bool {
    let client = reqwest::Client::new();
    match client
        .get(format!("{}/minio/health/live", RUSTFS_ENDPOINT))
        .timeout(Duration::from_secs(2))
        .send()
        .await
    {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

/// Skip test if S3 backend is not available
#[macro_export]
macro_rules! skip_if_no_s3 {
    () => {
        if !$crate::e2e::common::is_s3_backend_available().await {
            eprintln!("Skipping test: S3 backend not available");
            return;
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_random_payload_generation() {
        let payload = E2ETestEnv::random_payload(1024);
        assert_eq!(payload.len(), 1024);

        // Verify different calls produce different data
        let payload2 = E2ETestEnv::random_payload(1024);
        assert_ne!(payload, payload2);
    }

    #[tokio::test]
    async fn test_jwt_generation() {
        let token = E2ETestEnv::generate_test_jwt("test-user", 3600);
        assert!(!token.is_empty());

        // Token should have 3 parts (header.payload.signature)
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);
    }

    #[tokio::test]
    async fn test_expired_jwt_generation() {
        let token = E2ETestEnv::generate_expired_jwt("test-user");
        assert!(!token.is_empty());

        // Token should have 3 parts
        let parts: Vec<&str> = token.split('.').collect();
        assert_eq!(parts.len(), 3);
    }
}
