//! Authentication module
//!
//! Provides JWT and SigV4 authentication.
//! 
//! Note: JWT implementation can be referenced from Yatagarasu:
//! https://github.com/julianshen/yatagarasu/tree/master/src/auth

use async_trait::async_trait;
use thiserror::Error;

pub mod jwt;
pub mod sigv4;

/// Authentication errors
#[derive(Error, Debug)]
pub enum AuthError {
    #[error("Missing authentication")]
    MissingAuth,

    #[error("Invalid token: {0}")]
    InvalidToken(String),

    #[error("Token expired")]
    TokenExpired,

    #[error("Invalid signature")]
    InvalidSignature,

    #[error("JWKS fetch error: {0}")]
    JwksFetchError(String),
}

/// Authentication result containing claims
#[derive(Debug, Clone)]
pub struct AuthResult {
    pub subject: String,
    pub claims: std::collections::HashMap<String, serde_json::Value>,
}

/// Authenticator trait
#[async_trait]
pub trait Authenticator: Send + Sync {
    /// Authenticate a request
    async fn authenticate(&self, request: &AuthRequest) -> Result<AuthResult, AuthError>;
}

/// Authentication request context
#[derive(Debug)]
pub struct AuthRequest {
    pub headers: std::collections::HashMap<String, String>,
    pub query: Option<String>,
    pub method: String,
    pub path: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_result() {
        let result = AuthResult {
            subject: "user123".into(),
            claims: std::collections::HashMap::new(),
        };
        assert_eq!(result.subject, "user123");
    }
}
