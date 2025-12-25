//! OpenFGA Authorization
//!
//! Provides fine-grained access control using OpenFGA.
//! Reference: https://github.com/julianshen/yatagarasu/tree/master/src/authz/openfga

use super::{AuthzError, AuthzRequest, Authorizer};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// OpenFGA client configuration
#[derive(Debug, Clone)]
pub struct OpenFgaConfig {
    pub url: String,
    pub store_id: String,
    pub authorization_model_id: Option<String>,
}

/// OpenFGA Authorizer
pub struct OpenFgaAuthorizer {
    config: OpenFgaConfig,
    client: reqwest::Client,
}

/// OpenFGA check request
#[derive(Debug, Serialize)]
struct CheckRequest {
    tuple_key: TupleKey,
    #[serde(skip_serializing_if = "Option::is_none")]
    authorization_model_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct TupleKey {
    user: String,
    relation: String,
    object: String,
}

/// OpenFGA check response
#[derive(Debug, Deserialize)]
struct CheckResponse {
    allowed: bool,
}

impl OpenFgaAuthorizer {
    /// Create a new OpenFGA authorizer
    pub fn new(config: OpenFgaConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    /// Map action to OpenFGA relation
    fn action_to_relation(action: &str) -> &str {
        match action {
            "upload" | "write" | "put" => "writer",
            "create" => "creator",
            "delete" => "deleter",
            _ => "viewer",
        }
    }
}

#[async_trait]
impl Authorizer for OpenFgaAuthorizer {
    async fn authorize(&self, request: &AuthzRequest) -> Result<bool, AuthzError> {
        let url = format!(
            "{}/stores/{}/check",
            self.config.url, self.config.store_id
        );

        let check_request = CheckRequest {
            tuple_key: TupleKey {
                user: format!("user:{}", request.subject),
                relation: Self::action_to_relation(&request.action).to_string(),
                object: format!("bucket:{}", request.resource),
            },
            authorization_model_id: self.config.authorization_model_id.clone(),
        };

        let response = self
            .client
            .post(&url)
            .json(&check_request)
            .send()
            .await
            .map_err(|e| AuthzError::BackendError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(AuthzError::BackendError(format!(
                "OpenFGA returned status {}",
                response.status()
            )));
        }

        let check_response: CheckResponse = response
            .json()
            .await
            .map_err(|e| AuthzError::BackendError(e.to_string()))?;

        Ok(check_response.allowed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_action_to_relation() {
        assert_eq!(OpenFgaAuthorizer::action_to_relation("upload"), "writer");
        assert_eq!(OpenFgaAuthorizer::action_to_relation("write"), "writer");
        assert_eq!(OpenFgaAuthorizer::action_to_relation("create"), "creator");
        assert_eq!(OpenFgaAuthorizer::action_to_relation("unknown"), "viewer");
    }

    #[test]
    fn test_openfga_config() {
        let config = OpenFgaConfig {
            url: "http://localhost:8080".into(),
            store_id: "store123".into(),
            authorization_model_id: Some("model456".into()),
        };
        assert_eq!(config.store_id, "store123");
    }
}
