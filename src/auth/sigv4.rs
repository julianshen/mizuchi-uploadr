//! AWS SigV4 Authentication
//!
//! Validates AWS Signature Version 4 signed requests.

use super::{AuthError, AuthRequest, AuthResult, Authenticator};
use async_trait::async_trait;

/// SigV4 Authenticator
pub struct SigV4Authenticator {
    service: String,
    region: String,
    // In production, this would look up credentials from a store
    #[allow(dead_code)]
    credentials_store: std::collections::HashMap<String, String>,
}

impl SigV4Authenticator {
    /// Create a new SigV4 authenticator
    pub fn new(service: &str, region: &str) -> Self {
        Self {
            service: service.to_string(),
            region: region.to_string(),
            credentials_store: std::collections::HashMap::new(),
        }
    }

    /// Add credentials for an access key
    pub fn add_credentials(&mut self, access_key: &str, secret_key: &str) {
        self.credentials_store
            .insert(access_key.to_string(), secret_key.to_string());
    }
}

#[async_trait]
impl Authenticator for SigV4Authenticator {
    async fn authenticate(&self, request: &AuthRequest) -> Result<AuthResult, AuthError> {
        // Check for Authorization header
        let auth_header = request
            .headers
            .get("authorization")
            .ok_or(AuthError::MissingAuth)?;

        // Validate it starts with AWS4-HMAC-SHA256
        if !auth_header.starts_with("AWS4-HMAC-SHA256") {
            return Err(AuthError::InvalidToken(
                "Invalid SigV4 authorization header".into(),
            ));
        }

        // TODO: Implement full SigV4 validation
        // This is a placeholder for TDD - tests will drive implementation
        // Reference: https://github.com/julianshen/yatagarasu for patterns

        // For now, extract the access key from the Credential part
        let credential = auth_header
            .split(',')
            .find(|s| s.trim().starts_with("Credential="))
            .ok_or(AuthError::InvalidToken("Missing Credential".into()))?;

        let access_key = credential
            .trim()
            .strip_prefix("Credential=")
            .and_then(|s| s.split('/').next())
            .ok_or(AuthError::InvalidToken("Invalid Credential format".into()))?;

        Ok(AuthResult {
            subject: access_key.to_string(),
            claims: std::collections::HashMap::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sigv4_authenticator_creation() {
        let auth = SigV4Authenticator::new("s3", "us-east-1");
        assert_eq!(auth.service, "s3");
        assert_eq!(auth.region, "us-east-1");
    }

    #[tokio::test]
    async fn test_missing_auth_header() {
        let auth = SigV4Authenticator::new("s3", "us-east-1");
        let request = AuthRequest {
            headers: std::collections::HashMap::new(),
            query: None,
            method: "PUT".into(),
            path: "/bucket/key".into(),
        };

        let result = auth.authenticate(&request).await;
        assert!(matches!(result, Err(AuthError::MissingAuth)));
    }
}
