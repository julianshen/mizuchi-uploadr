//! OPA (Open Policy Agent) Authorization
//!
//! Provides policy-based access control using OPA.
//! Reference: https://github.com/julianshen/yatagarasu/tree/master/src/authz/opa
//!
//! # Example
//!
//! ```no_run
//! use mizuchi_uploadr::authz::opa::{OpaAuthorizer, OpaConfig};
//! use std::time::Duration;
//!
//! // Simple configuration
//! let config = OpaConfig {
//!     url: "http://localhost:8181".to_string(),
//!     policy_path: "mizuchi/allow".to_string(),
//!     timeout: Some(Duration::from_secs(5)),
//!     cache_ttl: Some(Duration::from_secs(60)),
//! };
//! let authorizer = OpaAuthorizer::new(config);
//!
//! // Or use builder pattern
//! let authorizer = OpaAuthorizer::builder()
//!     .url("http://localhost:8181")
//!     .policy_path("mizuchi/allow")
//!     .timeout(Duration::from_secs(5))
//!     .cache_ttl(Duration::from_secs(60))
//!     .build()
//!     .expect("valid config");
//! ```

use super::{Authorizer, AuthzError, AuthzRequest};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Default timeout for OPA requests (5 seconds)
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// Maximum cache size to prevent unbounded memory growth
const MAX_CACHE_SIZE: usize = 10_000;

/// OPA client configuration
#[derive(Debug, Clone)]
pub struct OpaConfig {
    /// OPA server URL (e.g., "http://localhost:8181")
    pub url: String,
    /// Policy path in OPA (e.g., "mizuchi/allow")
    pub policy_path: String,
    /// Request timeout (default: 5 seconds)
    pub timeout: Option<Duration>,
    /// Cache TTL for authorization decisions (None = no caching)
    pub cache_ttl: Option<Duration>,
}

/// Cached authorization decision
struct CachedDecision {
    allowed: bool,
    cached_at: Instant,
}

/// OPA Authorizer
///
/// Validates authorization using Open Policy Agent.
pub struct OpaAuthorizer {
    config: OpaConfig,
    client: reqwest::Client,
    /// Cache for authorization decisions (key = hash of request)
    cache: Arc<RwLock<HashMap<String, CachedDecision>>>,
}

/// Builder for OpaAuthorizer
#[derive(Default)]
pub struct OpaAuthorizerBuilder {
    url: Option<String>,
    policy_path: Option<String>,
    timeout: Option<Duration>,
    cache_ttl: Option<Duration>,
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

impl OpaAuthorizerBuilder {
    /// Set the OPA server URL
    pub fn url(mut self, url: &str) -> Self {
        self.url = Some(url.to_string());
        self
    }

    /// Set the policy path
    pub fn policy_path(mut self, path: &str) -> Self {
        self.policy_path = Some(path.to_string());
        self
    }

    /// Set the request timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set the cache TTL
    pub fn cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = Some(ttl);
        self
    }

    /// Build the OpaAuthorizer
    pub fn build(self) -> Result<OpaAuthorizer, AuthzError> {
        let url = self
            .url
            .ok_or_else(|| AuthzError::ConfigError("OPA URL is required".into()))?;
        let policy_path = self
            .policy_path
            .ok_or_else(|| AuthzError::ConfigError("OPA policy path is required".into()))?;

        let config = OpaConfig {
            url,
            policy_path,
            timeout: self.timeout,
            cache_ttl: self.cache_ttl,
        };

        Ok(OpaAuthorizer::new(config))
    }
}

impl OpaAuthorizer {
    /// Create a new OPA authorizer
    pub fn new(config: OpaConfig) -> Self {
        let timeout = config.timeout.unwrap_or(DEFAULT_TIMEOUT);
        let client = reqwest::Client::builder()
            .timeout(timeout)
            .build()
            .expect("Failed to build HTTP client");

        Self {
            config,
            client,
            cache: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Create a new builder for OpaAuthorizer
    pub fn builder() -> OpaAuthorizerBuilder {
        OpaAuthorizerBuilder::default()
    }

    /// Generate a cache key from the request
    fn cache_key(request: &AuthzRequest) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        request.subject.hash(&mut hasher);
        request.action.hash(&mut hasher);
        request.resource.hash(&mut hasher);
        // Hash context keys and values
        let mut context_keys: Vec<_> = request.context.keys().collect();
        context_keys.sort();
        for key in context_keys {
            key.hash(&mut hasher);
            if let Some(value) = request.context.get(key) {
                value.to_string().hash(&mut hasher);
            }
        }
        format!("{:x}", hasher.finish())
    }

    /// Check cache for a decision
    async fn check_cache(&self, key: &str) -> Option<bool> {
        let cache_ttl = self.config.cache_ttl?;
        let cache = self.cache.read().await;
        if let Some(cached) = cache.get(key) {
            if cached.cached_at.elapsed() < cache_ttl {
                return Some(cached.allowed);
            }
        }
        None
    }

    /// Store a decision in the cache
    async fn store_cache(&self, key: String, allowed: bool) {
        if self.config.cache_ttl.is_some() {
            let mut cache = self.cache.write().await;

            // Evict expired entries if cache is getting large
            if cache.len() >= MAX_CACHE_SIZE {
                let cache_ttl = self.config.cache_ttl.unwrap();
                cache.retain(|_, v| v.cached_at.elapsed() < cache_ttl);
            }

            // If still too large after cleanup, remove oldest entries
            if cache.len() >= MAX_CACHE_SIZE {
                // Find and remove oldest 10% of entries
                let to_remove = MAX_CACHE_SIZE / 10;
                let mut entries: Vec<_> = cache
                    .iter()
                    .map(|(k, v)| (k.clone(), v.cached_at))
                    .collect();
                entries.sort_by_key(|(_, t)| *t);
                for (key, _) in entries.into_iter().take(to_remove) {
                    cache.remove(&key);
                }
            }

            cache.insert(
                key,
                CachedDecision {
                    allowed,
                    cached_at: Instant::now(),
                },
            );
        }
    }

    /// Clear all cached authorization decisions
    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    /// Get the current cache size
    pub async fn cache_size(&self) -> usize {
        let cache = self.cache.read().await;
        cache.len()
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
        // Check cache first
        let cache_key = Self::cache_key(request);
        if let Some(cached_decision) = self.check_cache(&cache_key).await {
            #[cfg(feature = "tracing")]
            tracing::debug!(
                decision = %if cached_decision { "allow" } else { "deny" },
                "OPA authorization decision (cached)"
            );
            return Ok(cached_decision);
        }

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

        // Store in cache
        self.store_cache(cache_key, allowed).await;

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
            timeout: None,
            cache_ttl: None,
        };
        assert_eq!(config.url, "http://localhost:8181");
    }

    #[test]
    fn test_opa_config_with_options() {
        let config = OpaConfig {
            url: "http://localhost:8181".into(),
            policy_path: "mizuchi/allow".into(),
            timeout: Some(Duration::from_secs(10)),
            cache_ttl: Some(Duration::from_secs(60)),
        };
        assert_eq!(config.timeout, Some(Duration::from_secs(10)));
        assert_eq!(config.cache_ttl, Some(Duration::from_secs(60)));
    }

    #[test]
    fn test_builder_pattern() {
        let authorizer = OpaAuthorizer::builder()
            .url("http://localhost:8181")
            .policy_path("mizuchi/allow")
            .timeout(Duration::from_secs(5))
            .cache_ttl(Duration::from_secs(60))
            .build();
        assert!(authorizer.is_ok());
    }

    #[test]
    fn test_builder_missing_url() {
        let result = OpaAuthorizer::builder()
            .policy_path("mizuchi/allow")
            .build();
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_missing_policy_path() {
        let result = OpaAuthorizer::builder()
            .url("http://localhost:8181")
            .build();
        assert!(result.is_err());
    }
}
