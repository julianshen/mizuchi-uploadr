//! JWKS (JSON Web Key Set) Authentication
//!
//! Fetches and caches public keys from a JWKS endpoint for JWT validation.
//!
//! # Example
//!
//! ```no_run
//! use mizuchi_uploadr::auth::jwks::JwksAuthenticator;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Fetch keys from endpoint
//! let auth = JwksAuthenticator::new("https://auth.example.com/.well-known/jwks.json").await?;
//!
//! // Or parse from JSON directly
//! let json = r#"{"keys": [...]}"#;
//! let auth = JwksAuthenticator::from_json(json)?;
//! # Ok(())
//! # }
//! ```

use super::{AuthError, AuthRequest, AuthResult, Authenticator};
use async_trait::async_trait;
use jsonwebtoken::{decode, decode_header, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// A single JSON Web Key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Jwk {
    /// Key type (e.g., "RSA", "EC")
    pub kty: String,

    /// Key ID
    #[serde(default)]
    pub kid: Option<String>,

    /// Intended use ("sig" for signature)
    #[serde(default, rename = "use")]
    pub key_use: Option<String>,

    /// Algorithm (e.g., "RS256", "ES256")
    #[serde(default)]
    pub alg: Option<String>,

    // RSA parameters
    /// RSA modulus (base64url encoded)
    #[serde(default)]
    pub n: Option<String>,

    /// RSA exponent (base64url encoded)
    #[serde(default)]
    pub e: Option<String>,

    // EC parameters
    /// EC curve (e.g., "P-256")
    #[serde(default)]
    pub crv: Option<String>,

    /// EC x coordinate (base64url encoded)
    #[serde(default)]
    pub x: Option<String>,

    /// EC y coordinate (base64url encoded)
    #[serde(default)]
    pub y: Option<String>,
}

impl Jwk {
    /// Convert JWK to DecodingKey
    pub fn to_decoding_key(&self) -> Result<DecodingKey, AuthError> {
        match self.kty.as_str() {
            "RSA" => {
                let n = self
                    .n
                    .as_ref()
                    .ok_or_else(|| AuthError::InvalidToken("Missing RSA modulus (n)".into()))?;
                let e = self
                    .e
                    .as_ref()
                    .ok_or_else(|| AuthError::InvalidToken("Missing RSA exponent (e)".into()))?;

                DecodingKey::from_rsa_components(n, e)
                    .map_err(|e| AuthError::InvalidToken(format!("Invalid RSA key: {}", e)))
            }
            "EC" => {
                let x = self
                    .x
                    .as_ref()
                    .ok_or_else(|| AuthError::InvalidToken("Missing EC x coordinate".into()))?;
                let y = self
                    .y
                    .as_ref()
                    .ok_or_else(|| AuthError::InvalidToken("Missing EC y coordinate".into()))?;

                // from_ec_components expects base64url-encoded strings directly
                DecodingKey::from_ec_components(x, y)
                    .map_err(|e| AuthError::InvalidToken(format!("Invalid EC key: {}", e)))
            }
            other => Err(AuthError::InvalidToken(format!(
                "Unsupported key type: {}",
                other
            ))),
        }
    }

    /// Get the algorithm for this key
    pub fn algorithm(&self) -> Option<Algorithm> {
        self.alg.as_ref().and_then(|alg| match alg.as_str() {
            "RS256" => Some(Algorithm::RS256),
            "RS384" => Some(Algorithm::RS384),
            "RS512" => Some(Algorithm::RS512),
            "ES256" => Some(Algorithm::ES256),
            "ES384" => Some(Algorithm::ES384),
            _ => None,
        })
    }
}

/// JSON Web Key Set
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Jwks {
    pub keys: Vec<Jwk>,
}

/// Cached JWKS with metadata
struct CachedJwks {
    jwks: Jwks,
    fetched_at: std::time::Instant,
}

/// JWKS-based JWT Authenticator
///
/// Fetches keys from a JWKS endpoint and caches them.
///
/// # Example
///
/// ```no_run
/// use mizuchi_uploadr::auth::jwks::JwksAuthenticator;
/// use std::time::Duration;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let auth = JwksAuthenticator::new("https://auth.example.com/.well-known/jwks.json")
///     .await?
///     .with_cache_ttl(Duration::from_secs(3600))
///     .with_issuer("https://auth.example.com")
///     .with_audience("my-api");
/// # Ok(())
/// # }
/// ```
pub struct JwksAuthenticator {
    /// JWKS endpoint URL (if fetching remotely)
    endpoint: Option<String>,

    /// Cached keys
    cache: Arc<RwLock<CachedJwks>>,

    /// Cache TTL
    cache_ttl: Duration,

    /// HTTP client for fetching keys
    client: reqwest::Client,

    /// Required issuer (if set, tokens must have this issuer)
    required_issuer: Option<String>,

    /// Required audience (if set, tokens must have this audience)
    required_audience: Option<String>,
}

impl JwksAuthenticator {
    /// Create a new JWKS authenticator by fetching keys from an endpoint
    pub async fn new(endpoint: &str) -> Result<Self, AuthError> {
        let client = reqwest::Client::new();
        let jwks = Self::fetch_jwks(&client, endpoint).await?;

        Ok(Self {
            endpoint: Some(endpoint.to_string()),
            cache: Arc::new(RwLock::new(CachedJwks {
                jwks,
                fetched_at: std::time::Instant::now(),
            })),
            cache_ttl: Duration::from_secs(3600), // Default: 1 hour
            client,
            required_issuer: None,
            required_audience: None,
        })
    }

    /// Create a JWKS authenticator from JSON string
    pub fn from_json(json: &str) -> Result<Self, AuthError> {
        let jwks: Jwks =
            serde_json::from_str(json).map_err(|e| AuthError::InvalidToken(e.to_string()))?;

        Ok(Self {
            endpoint: None,
            cache: Arc::new(RwLock::new(CachedJwks {
                jwks,
                fetched_at: std::time::Instant::now(),
            })),
            cache_ttl: Duration::from_secs(3600),
            client: reqwest::Client::new(),
            required_issuer: None,
            required_audience: None,
        })
    }

    /// Set the cache TTL
    #[must_use]
    pub fn with_cache_ttl(mut self, ttl: Duration) -> Self {
        self.cache_ttl = ttl;
        self
    }

    /// Get the cache TTL
    pub fn cache_ttl(&self) -> Duration {
        self.cache_ttl
    }

    /// Set the required issuer (`iss` claim)
    ///
    /// Tokens without this issuer will be rejected.
    #[must_use]
    pub fn with_issuer(mut self, issuer: &str) -> Self {
        self.required_issuer = Some(issuer.to_string());
        self
    }

    /// Set the required audience (`aud` claim)
    ///
    /// Tokens without this audience will be rejected.
    #[must_use]
    pub fn with_audience(mut self, audience: &str) -> Self {
        self.required_audience = Some(audience.to_string());
        self
    }

    /// Check if the authenticator has any keys
    pub async fn has_keys(&self) -> bool {
        let cache = self.cache.read().await;
        !cache.jwks.keys.is_empty()
    }

    /// Find a key by its key ID (kid)
    pub async fn find_key(&self, kid: &str) -> Option<Jwk> {
        let cache = self.cache.read().await;
        cache
            .jwks
            .keys
            .iter()
            .find(|k| k.kid.as_deref() == Some(kid))
            .cloned()
    }

    /// Fetch JWKS from endpoint
    async fn fetch_jwks(client: &reqwest::Client, endpoint: &str) -> Result<Jwks, AuthError> {
        let response = client
            .get(endpoint)
            .send()
            .await
            .map_err(|e| AuthError::JwksFetchError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(AuthError::JwksFetchError(format!(
                "HTTP {}: {}",
                response.status(),
                response.status().canonical_reason().unwrap_or("Unknown")
            )));
        }

        let jwks: Jwks = response
            .json()
            .await
            .map_err(|e| AuthError::JwksFetchError(e.to_string()))?;

        Ok(jwks)
    }

    /// Refresh the JWKS cache if needed
    async fn refresh_if_needed(&self) -> Result<(), AuthError> {
        let needs_refresh = {
            let cache = self.cache.read().await;
            cache.fetched_at.elapsed() > self.cache_ttl
        };

        if needs_refresh {
            if let Some(endpoint) = &self.endpoint {
                let new_jwks = Self::fetch_jwks(&self.client, endpoint).await?;
                let mut cache = self.cache.write().await;
                cache.jwks = new_jwks;
                cache.fetched_at = std::time::Instant::now();
            }
        }

        Ok(())
    }

    /// Extract token from request
    fn extract_token(&self, request: &AuthRequest) -> Option<String> {
        // Try Authorization header first
        if let Some(auth) = request.headers.get("authorization") {
            if let Some(token) = auth.strip_prefix("Bearer ") {
                return Some(token.to_string());
            }
        }

        // Try query parameter
        if let Some(query) = &request.query {
            for pair in query.split('&') {
                if let Some(token) = pair.strip_prefix("token=") {
                    return Some(token.to_string());
                }
            }
        }

        None
    }
}

#[async_trait]
impl Authenticator for JwksAuthenticator {
    async fn authenticate(&self, request: &AuthRequest) -> Result<AuthResult, AuthError> {
        // Refresh cache if needed
        self.refresh_if_needed().await?;

        let token = self.extract_token(request).ok_or(AuthError::MissingAuth)?;

        // Decode header to get kid
        let header = decode_header(&token)
            .map_err(|e| AuthError::InvalidToken(format!("Invalid token header: {}", e)))?;

        // Find the key
        let jwk = if let Some(kid) = &header.kid {
            self.find_key(kid)
                .await
                .ok_or_else(|| AuthError::InvalidToken(format!("Key not found: {}", kid)))?
        } else {
            // No kid in token, use first key
            let cache = self.cache.read().await;
            cache
                .jwks
                .keys
                .first()
                .cloned()
                .ok_or_else(|| AuthError::InvalidToken("No keys available".into()))?
        };

        // Get decoding key
        let decoding_key = jwk.to_decoding_key()?;

        // Determine algorithm
        let algorithm = jwk
            .algorithm()
            .or(match header.alg {
                jsonwebtoken::Algorithm::RS256 => Some(Algorithm::RS256),
                jsonwebtoken::Algorithm::RS384 => Some(Algorithm::RS384),
                jsonwebtoken::Algorithm::RS512 => Some(Algorithm::RS512),
                jsonwebtoken::Algorithm::ES256 => Some(Algorithm::ES256),
                jsonwebtoken::Algorithm::ES384 => Some(Algorithm::ES384),
                _ => None,
            })
            .ok_or_else(|| AuthError::InvalidToken("Unsupported algorithm".into()))?;

        // Create validation
        let mut validation = Validation::new(algorithm);
        validation.validate_exp = true;

        // Configure issuer validation if set
        if let Some(issuer) = &self.required_issuer {
            validation.set_issuer(&[issuer]);
        }

        // Configure audience validation if set
        if let Some(audience) = &self.required_audience {
            validation.set_audience(&[audience]);
            validation.validate_aud = true;
        } else {
            validation.validate_aud = false;
        }

        // Decode and validate token
        let token_data = decode::<super::jwt::Claims>(&token, &decoding_key, &validation).map_err(
            |e| match e.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
                jsonwebtoken::errors::ErrorKind::InvalidSignature => AuthError::InvalidSignature,
                _ => AuthError::InvalidToken(e.to_string()),
            },
        )?;

        // Build result
        let mut claims_map = HashMap::new();
        if let Some(iss) = &token_data.claims.iss {
            claims_map.insert("iss".into(), serde_json::Value::String(iss.clone()));
        }
        if let Some(aud) = &token_data.claims.aud {
            claims_map.insert("aud".into(), serde_json::Value::String(aud.clone()));
        }

        Ok(AuthResult {
            subject: token_data.claims.sub,
            claims: claims_map,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_jwks() {
        let json = r#"{
            "keys": [
                {
                    "kty": "RSA",
                    "kid": "key-1",
                    "use": "sig",
                    "alg": "RS256",
                    "n": "0vx7agoebGcQSuuPiLJXZptN9nndrQmbXEps2aiAFbWhM78LhWx4cbbfAAtVT86zwu1RK7aPFFxuhDR1L6tSoc_BJECPebWKRXjBZCiFV4n3oknjhMstn64tZ_2W-5JsGY4Hc5n9yBXArwl93lqt7_RN5w6Cf0h4QyQ5v-65YGjQR0_FDW2QvzqY368QQMicAtaSqzs8KJZgnYb9c7d0zgdAZHzu6qMQvRL5hajrn1n91CbOpbISD08qNLyrdkt-bFTWhAI4vMQFh6WeZu0fM4lFd2NcRwr3XPksINHaQ-G_xBniIqbw0Ls1jF44-csFCur-kEgU8awapJzKnqDKgw",
                    "e": "AQAB"
                }
            ]
        }"#;

        let jwks: Jwks = serde_json::from_str(json).unwrap();
        assert_eq!(jwks.keys.len(), 1);
        assert_eq!(jwks.keys[0].kid, Some("key-1".to_string()));
    }

    #[test]
    fn test_jwk_to_decoding_key() {
        let jwk = Jwk {
            kty: "RSA".to_string(),
            kid: Some("key-1".to_string()),
            key_use: Some("sig".to_string()),
            alg: Some("RS256".to_string()),
            n: Some("0vx7agoebGcQSuuPiLJXZptN9nndrQmbXEps2aiAFbWhM78LhWx4cbbfAAtVT86zwu1RK7aPFFxuhDR1L6tSoc_BJECPebWKRXjBZCiFV4n3oknjhMstn64tZ_2W-5JsGY4Hc5n9yBXArwl93lqt7_RN5w6Cf0h4QyQ5v-65YGjQR0_FDW2QvzqY368QQMicAtaSqzs8KJZgnYb9c7d0zgdAZHzu6qMQvRL5hajrn1n91CbOpbISD08qNLyrdkt-bFTWhAI4vMQFh6WeZu0fM4lFd2NcRwr3XPksINHaQ-G_xBniIqbw0Ls1jF44-csFCur-kEgU8awapJzKnqDKgw".to_string()),
            e: Some("AQAB".to_string()),
            crv: None,
            x: None,
            y: None,
        };

        let result = jwk.to_decoding_key();
        assert!(result.is_ok());
    }
}
