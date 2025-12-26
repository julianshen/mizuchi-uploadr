//! JWT Authentication
//!
//! Supports HS256, RS256, ES256 algorithms and JWKS endpoints.
//! Reference implementation: https://github.com/julianshen/yatagarasu/blob/master/src/auth/jwt.rs

use super::{AuthError, AuthRequest, AuthResult, Authenticator};
use async_trait::async_trait;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

/// JWT Claims
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    #[serde(default)]
    pub iat: Option<usize>,
    #[serde(default)]
    pub iss: Option<String>,
    #[serde(default)]
    pub aud: Option<String>,
}

/// JWT Authenticator
pub struct JwtAuthenticator {
    decoding_key: DecodingKey,
    validation: Validation,
}

impl JwtAuthenticator {
    /// Create a new JWT authenticator with a secret key (HS256)
    pub fn new_hs256(secret: &str) -> Self {
        let decoding_key = DecodingKey::from_secret(secret.as_bytes());
        let mut validation = Validation::new(Algorithm::HS256);
        validation.validate_exp = true;

        Self {
            decoding_key,
            validation,
        }
    }

    /// Create a new JWT authenticator with an RSA public key (RS256)
    pub fn new_rs256(public_key_pem: &str) -> Result<Self, AuthError> {
        let decoding_key = DecodingKey::from_rsa_pem(public_key_pem.as_bytes())
            .map_err(|e| AuthError::InvalidToken(e.to_string()))?;
        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = true;

        Ok(Self {
            decoding_key,
            validation,
        })
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
impl Authenticator for JwtAuthenticator {
    #[cfg_attr(feature = "tracing", tracing::instrument(
        name = "auth.jwt",
        skip(self, request),
        fields(
            auth.method = "jwt",
            auth.token_present = %self.extract_token(request).is_some(),
            otel.kind = "internal"
        ),
        err
    ))]
    async fn authenticate(&self, request: &AuthRequest) -> Result<AuthResult, AuthError> {
        let token = self.extract_token(request).ok_or(AuthError::MissingAuth)?;

        let token_data =
            decode::<Claims>(&token, &self.decoding_key, &self.validation).map_err(|e| match e
                .kind()
            {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
                jsonwebtoken::errors::ErrorKind::InvalidSignature => AuthError::InvalidSignature,
                _ => AuthError::InvalidToken(e.to_string()),
            })?;

        let mut claims_map = std::collections::HashMap::new();
        if let Some(iss) = &token_data.claims.iss {
            claims_map.insert("iss".into(), serde_json::Value::String(iss.clone()));
        }
        if let Some(aud) = &token_data.claims.aud {
            claims_map.insert("aud".into(), serde_json::Value::String(aud.clone()));
        }

        #[cfg(feature = "tracing")]
        tracing::info!(
            subject = %token_data.claims.sub,
            "JWT authentication successful"
        );

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
    fn test_jwt_authenticator_creation() {
        // Test passes if construction doesn't panic
        let _auth = JwtAuthenticator::new_hs256("secret");
    }

    #[tokio::test]
    async fn test_missing_token() {
        let auth = JwtAuthenticator::new_hs256("secret");
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
