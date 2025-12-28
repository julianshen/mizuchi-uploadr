//! S3 Credentials Module
//!
//! Provides credential loading from various sources using a trait-based design.
//!
//! # Design
//!
//! Uses a trait-based approach for flexibility:
//! - `CredentialsProvider` trait defines the interface
//! - Multiple implementations for different credential sources
//!
//! # Implementations
//!
//! - `StaticCredentials` - Credentials from configuration
//! - `EnvironmentCredentials` - Credentials from environment variables
//!
//! # Example
//!
//! ```
//! use mizuchi_uploadr::s3::{Credentials, StaticCredentials, CredentialsProviderTrait};
//!
//! // Create static credentials
//! let provider = StaticCredentials::new("access-key", "secret-key");
//!
//! // Access credentials through the trait method
//! let creds = provider.credentials();
//! assert_eq!(creds.access_key_id(), "access-key");
//! assert_eq!(creds.secret_access_key(), "secret-key");
//! ```

use crate::config::S3Config;
use thiserror::Error;

/// Credential loading errors
#[derive(Error, Debug)]
pub enum CredentialsError {
    #[error("Missing credentials: {0}")]
    MissingCredentials(String),

    #[error("Invalid credentials: {0}")]
    InvalidCredentials(String),

    #[error("Environment error: {0}")]
    EnvironmentError(String),
}

/// Credentials for AWS authentication
///
/// This struct holds the actual credential values.
/// It's returned by `CredentialsProvider` implementations.
#[derive(Debug, Clone)]
pub struct Credentials {
    access_key_id: String,
    secret_access_key: String,
    session_token: Option<String>,
}

impl Credentials {
    /// Create new credentials
    pub fn new(access_key_id: impl Into<String>, secret_access_key: impl Into<String>) -> Self {
        Self {
            access_key_id: access_key_id.into(),
            secret_access_key: secret_access_key.into(),
            session_token: None,
        }
    }

    /// Create credentials with session token (for temporary credentials)
    pub fn with_session_token(
        access_key_id: impl Into<String>,
        secret_access_key: impl Into<String>,
        session_token: impl Into<String>,
    ) -> Self {
        Self {
            access_key_id: access_key_id.into(),
            secret_access_key: secret_access_key.into(),
            session_token: Some(session_token.into()),
        }
    }

    /// Get the access key ID
    pub fn access_key_id(&self) -> &str {
        &self.access_key_id
    }

    /// Get the secret access key
    pub fn secret_access_key(&self) -> &str {
        &self.secret_access_key
    }

    /// Get the session token (if any)
    pub fn session_token(&self) -> Option<&str> {
        self.session_token.as_deref()
    }
}

/// Trait for credential providers
///
/// Implement this trait to create custom credential loading mechanisms.
pub trait CredentialsProviderTrait: Send + Sync {
    /// Get credentials from this provider
    fn credentials(&self) -> &Credentials;
}

/// Wrapper that provides factory methods for creating credential providers
///
/// This struct doesn't hold credentials itself but provides static methods
/// to create appropriate providers.
pub struct CredentialsProvider;

impl CredentialsProvider {
    /// Load credentials from environment variables
    ///
    /// Looks for:
    /// - `AWS_ACCESS_KEY_ID`
    /// - `AWS_SECRET_ACCESS_KEY`
    /// - `AWS_SESSION_TOKEN` (optional)
    pub async fn from_env() -> Result<Credentials, CredentialsError> {
        let access_key = std::env::var("AWS_ACCESS_KEY_ID").map_err(|_| {
            CredentialsError::MissingCredentials("AWS_ACCESS_KEY_ID not set".into())
        })?;

        let secret_key = std::env::var("AWS_SECRET_ACCESS_KEY").map_err(|_| {
            CredentialsError::MissingCredentials("AWS_SECRET_ACCESS_KEY not set".into())
        })?;

        let session_token = std::env::var("AWS_SESSION_TOKEN").ok();

        Ok(match session_token {
            Some(token) => Credentials::with_session_token(access_key, secret_key, token),
            None => Credentials::new(access_key, secret_key),
        })
    }

    /// Load credentials from S3Config
    ///
    /// Uses the `access_key` and `secret_key` fields from the configuration.
    pub fn from_config(config: &S3Config) -> Result<Credentials, CredentialsError> {
        let access_key = config.access_key.as_ref().ok_or_else(|| {
            CredentialsError::MissingCredentials("access_key not set in config".into())
        })?;

        let secret_key = config.secret_key.as_ref().ok_or_else(|| {
            CredentialsError::MissingCredentials("secret_key not set in config".into())
        })?;

        Ok(Credentials::new(access_key.clone(), secret_key.clone()))
    }
}

/// Static credentials provider
///
/// Holds credentials directly. Useful for testing or when credentials
/// are known at compile time.
#[derive(Debug, Clone)]
pub struct StaticCredentials {
    credentials: Credentials,
}

impl StaticCredentials {
    /// Create a new static credentials provider
    pub fn new(access_key_id: impl Into<String>, secret_access_key: impl Into<String>) -> Self {
        Self {
            credentials: Credentials::new(access_key_id, secret_access_key),
        }
    }
}

impl CredentialsProviderTrait for StaticCredentials {
    fn credentials(&self) -> &Credentials {
        &self.credentials
    }
}

/// Environment credentials provider
///
/// Loads credentials from environment variables when created.
#[derive(Debug, Clone)]
pub struct EnvironmentCredentials {
    credentials: Credentials,
}

impl EnvironmentCredentials {
    /// Create a new environment credentials provider
    ///
    /// Loads credentials from environment variables immediately.
    pub async fn new() -> Result<Self, CredentialsError> {
        let credentials = CredentialsProvider::from_env().await?;
        Ok(Self { credentials })
    }
}

impl CredentialsProviderTrait for EnvironmentCredentials {
    fn credentials(&self) -> &Credentials {
        &self.credentials
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_credentials_creation() {
        let creds = Credentials::new("access", "secret");
        assert_eq!(creds.access_key_id(), "access");
        assert_eq!(creds.secret_access_key(), "secret");
        assert!(creds.session_token().is_none());
    }

    #[test]
    fn test_credentials_with_session_token() {
        let creds = Credentials::with_session_token("access", "secret", "token");
        assert_eq!(creds.access_key_id(), "access");
        assert_eq!(creds.secret_access_key(), "secret");
        assert_eq!(creds.session_token(), Some("token"));
    }

    #[test]
    fn test_static_credentials() {
        let provider = StaticCredentials::new("static-access", "static-secret");
        assert_eq!(provider.credentials().access_key_id(), "static-access");
        assert_eq!(provider.credentials().secret_access_key(), "static-secret");
    }

    #[test]
    fn test_from_config_missing_access_key() {
        let config = S3Config {
            bucket: "test".into(),
            region: "us-east-1".into(),
            endpoint: None,
            access_key: None,
            secret_key: Some("secret".into()),
        };

        let result = CredentialsProvider::from_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_config_missing_secret_key() {
        let config = S3Config {
            bucket: "test".into(),
            region: "us-east-1".into(),
            endpoint: None,
            access_key: Some("access".into()),
            secret_key: None,
        };

        let result = CredentialsProvider::from_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_config_success() {
        let config = S3Config {
            bucket: "test".into(),
            region: "us-east-1".into(),
            endpoint: None,
            access_key: Some("config-access".into()),
            secret_key: Some("config-secret".into()),
        };

        let result = CredentialsProvider::from_config(&config);
        assert!(result.is_ok());
        let creds = result.unwrap();
        assert_eq!(creds.access_key_id(), "config-access");
        assert_eq!(creds.secret_access_key(), "config-secret");
    }
}
