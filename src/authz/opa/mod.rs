//! OPA (Open Policy Agent) Authorization
//!
//! Provides policy-based access control using OPA.
//! Reference: https://github.com/julianshen/yatagarasu/tree/master/src/authz/opa

use super::{Authorizer, AuthzError, AuthzRequest};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// OPA client configuration
#[derive(Debug, Clone)]
pub struct OpaConfig {
    pub url: String,
    pub policy_path: String,
}

/// OPA Authorizer
pub struct OpaAuthorizer {
    config: OpaConfig,
    client: reqwest::Client,
}

/// OPA request input
#[derive(Debug, Serialize)]
struct OpaInput {
    input: OpaInputData,
}

#[derive(Debug, Serialize)]
struct OpaInputData {
    subject: String,
    action: String,
    resource: String,
    #[serde(flatten)]
    context: std::collections::HashMap<String, serde_json::Value>,
}

/// OPA response
#[derive(Debug, Deserialize)]
struct OpaResponse {
    result: Option<bool>,
}

impl OpaAuthorizer {
    /// Create a new OPA authorizer
    pub fn new(config: OpaConfig) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl Authorizer for OpaAuthorizer {
    #[cfg_attr(feature = "tracing", tracing::instrument(
        name = "authz.opa",
        skip(self, request),
        fields(
            authz.method = "opa",
            authz.action = %request.action,
            authz.resource_type = %extract_resource_type(&request.resource),
            otel.kind = "internal"
        ),
        err
    ))]
    async fn authorize(&self, request: &AuthzRequest) -> Result<bool, AuthzError> {
        let url = format!("{}/v1/data/{}", self.config.url, self.config.policy_path);

        let input = OpaInput {
            input: OpaInputData {
                subject: request.subject.clone(),
                action: request.action.clone(),
                resource: request.resource.clone(),
                context: request.context.clone(),
            },
        };

        let response = self
            .client
            .post(&url)
            .json(&input)
            .send()
            .await
            .map_err(|e| AuthzError::BackendError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(AuthzError::BackendError(format!(
                "OPA returned status {}",
                response.status()
            )));
        }

        let opa_response: OpaResponse = response
            .json()
            .await
            .map_err(|e| AuthzError::BackendError(e.to_string()))?;

        let allowed = opa_response.result.unwrap_or(false);

        #[cfg(feature = "tracing")]
        tracing::info!(
            decision = %if allowed { "allow" } else { "deny" },
            "OPA authorization decision"
        );

        Ok(allowed)
    }
}

/// Extract resource type from resource path (no PII)
#[cfg(feature = "tracing")]
fn extract_resource_type(resource: &str) -> String {
    if resource.starts_with("bucket/") {
        "bucket".to_string()
    } else if resource.contains('/') {
        "object".to_string()
    } else {
        "unknown".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_opa_config() {
        let config = OpaConfig {
            url: "http://localhost:8181".into(),
            policy_path: "mizuchi/allow".into(),
        };
        assert_eq!(config.url, "http://localhost:8181");
    }
}
