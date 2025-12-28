//! OpenFGA Authorization
//!
//! Provides fine-grained access control using OpenFGA.
//! Reference: https://github.com/julianshen/yatagarasu/tree/master/src/authz/openfga
//!
//! # Example
//!
//! ```no_run
//! use mizuchi_uploadr::authz::openfga::{OpenFgaAuthorizer, OpenFgaConfig};
//! use std::time::Duration;
//!
//! // Simple configuration
//! let config = OpenFgaConfig {
//!     url: "http://localhost:8080".to_string(),
//!     store_id: "my-store".to_string(),
//!     authorization_model_id: Some("model-123".to_string()),
//!     timeout: Some(Duration::from_secs(5)),
//!     cache_ttl: Some(Duration::from_secs(60)),
//! };
//! let authorizer = OpenFgaAuthorizer::new(config);
//!
//! // Or use builder pattern
//! let authorizer = OpenFgaAuthorizer::builder()
//!     .url("http://localhost:8080")
//!     .store_id("my-store")
//!     .authorization_model_id("model-123")
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

/// Default timeout for OpenFGA requests (5 seconds)
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// Maximum cache size to prevent unbounded memory growth
const MAX_CACHE_SIZE: usize = 10_000;

/// OpenFGA client configuration
#[derive(Debug, Clone)]
pub struct OpenFgaConfig {
    /// OpenFGA server URL (e.g., "http://localhost:8080")
    pub url: String,
    /// Store ID for the authorization model
    pub store_id: String,
    /// Authorization model ID (optional, uses latest if not set)
    pub authorization_model_id: Option<String>,
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

/// OpenFGA Authorizer
///
/// Validates authorization using OpenFGA relationship-based access control.
pub struct OpenFgaAuthorizer {
    config: OpenFgaConfig,
    client: reqwest::Client,
    /// Cache for authorization decisions (key = hash of request)
    cache: Arc<RwLock<HashMap<String, CachedDecision>>>,
}

/// Builder for OpenFgaAuthorizer
#[derive(Default)]
pub struct OpenFgaAuthorizerBuilder {
    url: Option<String>,
    store_id: Option<String>,
    authorization_model_id: Option<String>,
    timeout: Option<Duration>,
    cache_ttl: Option<Duration>,
}

/// OpenFGA check request
#[derive(Debug, Serialize)]
struct CheckRequest {
    tuple_key: TupleKey,
    #[serde(skip_serializing_if = "Option::is_none")]
    authorization_model_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct TupleKey {
    user: String,
    relation: String,
    object: String,
}

impl TupleKey {
    /// Create a tuple key from an authorization request
    fn from_request(request: &AuthzRequest) -> Self {
        Self {
            user: format!("user:{}", request.subject),
            relation: OpenFgaAuthorizer::action_to_relation(&request.action).to_string(),
            object: format!("bucket:{}", request.resource),
        }
    }
}

/// OpenFGA check response
#[derive(Debug, Deserialize)]
struct CheckResponse {
    allowed: bool,
}

/// OpenFGA batch check request
#[derive(Debug, Serialize)]
struct BatchCheckRequest {
    checks: Vec<BatchCheckItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    authorization_model_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct BatchCheckItem {
    tuple_key: TupleKey,
}

/// OpenFGA batch check response
#[derive(Debug, Deserialize)]
struct BatchCheckResponse {
    results: Vec<BatchCheckResult>,
}

#[derive(Debug, Deserialize)]
struct BatchCheckResult {
    allowed: bool,
}

impl OpenFgaAuthorizerBuilder {
    /// Set the OpenFGA server URL
    pub fn url(mut self, url: &str) -> Self {
        self.url = Some(url.to_string());
        self
    }

    /// Set the store ID
    pub fn store_id(mut self, id: &str) -> Self {
        self.store_id = Some(id.to_string());
        self
    }

    /// Set the authorization model ID
    pub fn authorization_model_id(mut self, id: &str) -> Self {
        self.authorization_model_id = Some(id.to_string());
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

    /// Build the OpenFgaAuthorizer
    pub fn build(self) -> Result<OpenFgaAuthorizer, AuthzError> {
        let url = self
            .url
            .ok_or_else(|| AuthzError::ConfigError("OpenFGA URL is required".into()))?;
        let store_id = self
            .store_id
            .ok_or_else(|| AuthzError::ConfigError("OpenFGA store_id is required".into()))?;

        let config = OpenFgaConfig {
            url,
            store_id,
            authorization_model_id: self.authorization_model_id,
            timeout: self.timeout,
            cache_ttl: self.cache_ttl,
        };

        Ok(OpenFgaAuthorizer::new(config))
    }
}

impl OpenFgaAuthorizer {
    /// Create a new OpenFGA authorizer
    pub fn new(config: OpenFgaConfig) -> Self {
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

    /// Create a new builder for OpenFgaAuthorizer
    pub fn builder() -> OpenFgaAuthorizerBuilder {
        OpenFgaAuthorizerBuilder::default()
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

    /// Generate a cache key from the request
    fn cache_key(request: &AuthzRequest) -> String {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        request.subject.hash(&mut hasher);
        request.action.hash(&mut hasher);
        request.resource.hash(&mut hasher);
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

    /// Perform a batch check for multiple authorization requests
    ///
    /// This is more efficient than making individual `authorize()` calls when you need
    /// to check multiple authorizations at once, reducing network round-trips.
    ///
    /// # Arguments
    /// * `requests` - Slice of authorization requests to check
    ///
    /// # Returns
    /// A vector of booleans in the same order as the input requests,
    /// where `true` indicates the authorization is allowed.
    ///
    /// # Note
    /// Batch checks are NOT cached. Use individual `authorize()` calls if you
    /// need caching for frequently repeated checks.
    pub async fn batch_check(&self, requests: &[AuthzRequest]) -> Result<Vec<bool>, AuthzError> {
        let url = format!(
            "{}/stores/{}/batch-check",
            self.config.url, self.config.store_id
        );

        let checks: Vec<BatchCheckItem> = requests
            .iter()
            .map(|r| BatchCheckItem {
                tuple_key: TupleKey::from_request(r),
            })
            .collect();

        let batch_request = BatchCheckRequest {
            checks,
            authorization_model_id: self.config.authorization_model_id.clone(),
        };

        let response = self
            .client
            .post(&url)
            .json(&batch_request)
            .send()
            .await
            .map_err(|e| AuthzError::BackendError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(AuthzError::BackendError(format!(
                "OpenFGA returned status {}",
                response.status()
            )));
        }

        let batch_response: BatchCheckResponse = response
            .json()
            .await
            .map_err(|e| AuthzError::BackendError(e.to_string()))?;

        Ok(batch_response
            .results
            .into_iter()
            .map(|r| r.allowed)
            .collect())
    }
}

#[async_trait]
impl Authorizer for OpenFgaAuthorizer {
    #[cfg_attr(feature = "tracing", tracing::instrument(
        name = "authz.openfga",
        skip(self, request),
        fields(
            authz.method = "openfga",
            authz.action = %request.action,
            authz.relation = %Self::action_to_relation(&request.action),
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
                "OpenFGA authorization decision (cached)"
            );
            return Ok(cached_decision);
        }

        let url = format!("{}/stores/{}/check", self.config.url, self.config.store_id);

        let check_request = CheckRequest {
            tuple_key: TupleKey::from_request(request),
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

        // Store in cache
        self.store_cache(cache_key, check_response.allowed).await;

        #[cfg(feature = "tracing")]
        tracing::info!(
            decision = %if check_response.allowed { "allow" } else { "deny" },
            "OpenFGA authorization decision"
        );

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
            timeout: None,
            cache_ttl: None,
        };
        assert_eq!(config.store_id, "store123");
    }

    #[test]
    fn test_openfga_config_with_options() {
        let config = OpenFgaConfig {
            url: "http://localhost:8080".into(),
            store_id: "store123".into(),
            authorization_model_id: Some("model456".into()),
            timeout: Some(Duration::from_secs(10)),
            cache_ttl: Some(Duration::from_secs(60)),
        };
        assert_eq!(config.timeout, Some(Duration::from_secs(10)));
        assert_eq!(config.cache_ttl, Some(Duration::from_secs(60)));
    }

    #[test]
    fn test_builder_pattern() {
        let authorizer = OpenFgaAuthorizer::builder()
            .url("http://localhost:8080")
            .store_id("store123")
            .authorization_model_id("model456")
            .timeout(Duration::from_secs(5))
            .cache_ttl(Duration::from_secs(60))
            .build();
        assert!(authorizer.is_ok());
    }

    #[test]
    fn test_builder_missing_url() {
        let result = OpenFgaAuthorizer::builder().store_id("store123").build();
        assert!(result.is_err());
    }

    #[test]
    fn test_builder_missing_store_id() {
        let result = OpenFgaAuthorizer::builder()
            .url("http://localhost:8080")
            .build();
        assert!(result.is_err());
    }
}
