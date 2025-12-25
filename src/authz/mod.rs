//! Authorization module
//!
//! Provides OPA and OpenFGA based authorization.
//!
//! Reference implementations from Yatagarasu:
//! - OPA: https://github.com/julianshen/yatagarasu/tree/master/src/authz/opa
//! - OpenFGA: https://github.com/julianshen/yatagarasu/tree/master/src/authz/openfga

use async_trait::async_trait;
use thiserror::Error;

pub mod opa;
pub mod openfga;

#[cfg(feature = "tracing")]
pub mod opa_tracing;

/// Authorization errors
#[derive(Error, Debug)]
pub enum AuthzError {
    #[error("Access denied")]
    AccessDenied,

    #[error("Policy error: {0}")]
    PolicyError(String),

    #[error("Backend error: {0}")]
    BackendError(String),
}

/// Authorization request
#[derive(Debug, Clone)]
pub struct AuthzRequest {
    pub subject: String,
    pub action: String,
    pub resource: String,
    pub context: std::collections::HashMap<String, serde_json::Value>,
}

/// Authorizer trait
#[async_trait]
pub trait Authorizer: Send + Sync {
    /// Check if the request is authorized
    async fn authorize(&self, request: &AuthzRequest) -> Result<bool, AuthzError>;
}

/// No-op authorizer that always allows
pub struct AllowAllAuthorizer;

#[async_trait]
impl Authorizer for AllowAllAuthorizer {
    async fn authorize(&self, _request: &AuthzRequest) -> Result<bool, AuthzError> {
        Ok(true)
    }
}

/// No-op authorizer that always denies
pub struct DenyAllAuthorizer;

#[async_trait]
impl Authorizer for DenyAllAuthorizer {
    async fn authorize(&self, _request: &AuthzRequest) -> Result<bool, AuthzError> {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_request() -> AuthzRequest {
        AuthzRequest {
            subject: "user123".into(),
            action: "upload".into(),
            resource: "bucket/key".into(),
            context: std::collections::HashMap::new(),
        }
    }

    #[tokio::test]
    async fn test_allow_all() {
        let authz = AllowAllAuthorizer;
        let result = authz.authorize(&test_request()).await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    async fn test_deny_all() {
        let authz = DenyAllAuthorizer;
        let result = authz.authorize(&test_request()).await.unwrap();
        assert!(!result);
    }
}
