//! JWT Authentication Integration Tests
//!
//! TDD RED Phase: These tests define expected JWT authentication behavior.
//! Tests cover HS256/RS256/ES256 algorithms, JWKS endpoints, and error cases.

#[cfg(test)]
mod tests {
    use mizuchi_uploadr::auth::jwt::{Claims, JwtAuthenticator};
    use mizuchi_uploadr::auth::{AuthError, AuthRequest, Authenticator};
    use std::collections::HashMap;

    // ========================================================================
    // Helper: Create test tokens
    // ========================================================================

    fn create_hs256_token(secret: &str, claims: &Claims) -> String {
        use jsonwebtoken::{encode, EncodingKey, Header};
        encode(
            &Header::default(),
            claims,
            &EncodingKey::from_secret(secret.as_bytes()),
        )
        .unwrap()
    }

    fn create_request_with_token(token: &str) -> AuthRequest {
        let mut headers = HashMap::new();
        headers.insert("authorization".to_string(), format!("Bearer {}", token));
        AuthRequest {
            headers,
            query: None,
            method: "PUT".to_string(),
            path: "/bucket/key".to_string(),
        }
    }

    fn valid_claims() -> Claims {
        Claims {
            sub: "user123".to_string(),
            exp: (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp() as usize,
            iat: Some(chrono::Utc::now().timestamp() as usize),
            iss: Some("test-issuer".to_string()),
            aud: Some("test-audience".to_string()),
        }
    }

    fn expired_claims() -> Claims {
        Claims {
            sub: "user123".to_string(),
            exp: (chrono::Utc::now() - chrono::Duration::hours(1)).timestamp() as usize,
            iat: Some(
                (chrono::Utc::now() - chrono::Duration::hours(2)).timestamp() as usize,
            ),
            iss: None,
            aud: None,
        }
    }

    // ========================================================================
    // TEST: HS256 Algorithm
    // ========================================================================

    #[tokio::test]
    async fn test_valid_hs256_token_accepted() {
        let secret = "super-secret-key-for-testing";
        let auth = JwtAuthenticator::new_hs256(secret);

        let claims = valid_claims();
        let token = create_hs256_token(secret, &claims);
        let request = create_request_with_token(&token);

        let result = auth.authenticate(&request).await;
        assert!(result.is_ok(), "Valid HS256 token should be accepted");

        let auth_result = result.unwrap();
        assert_eq!(auth_result.subject, "user123");
    }

    #[tokio::test]
    async fn test_invalid_hs256_signature_rejected() {
        let auth = JwtAuthenticator::new_hs256("correct-secret");

        let claims = valid_claims();
        let token = create_hs256_token("wrong-secret", &claims);
        let request = create_request_with_token(&token);

        let result = auth.authenticate(&request).await;
        assert!(
            matches!(result, Err(AuthError::InvalidSignature)),
            "Token with wrong secret should be rejected"
        );
    }

    #[tokio::test]
    async fn test_expired_token_rejected() {
        let secret = "test-secret";
        let auth = JwtAuthenticator::new_hs256(secret);

        let claims = expired_claims();
        let token = create_hs256_token(secret, &claims);
        let request = create_request_with_token(&token);

        let result = auth.authenticate(&request).await;
        assert!(
            matches!(result, Err(AuthError::TokenExpired)),
            "Expired token should be rejected"
        );
    }

    #[tokio::test]
    async fn test_malformed_token_rejected() {
        let auth = JwtAuthenticator::new_hs256("secret");

        let request = create_request_with_token("not-a-valid-jwt-token");

        let result = auth.authenticate(&request).await;
        assert!(
            matches!(result, Err(AuthError::InvalidToken(_))),
            "Malformed token should be rejected"
        );
    }

    #[tokio::test]
    async fn test_missing_token_rejected() {
        let auth = JwtAuthenticator::new_hs256("secret");

        let request = AuthRequest {
            headers: HashMap::new(),
            query: None,
            method: "PUT".to_string(),
            path: "/bucket/key".to_string(),
        };

        let result = auth.authenticate(&request).await;
        assert!(
            matches!(result, Err(AuthError::MissingAuth)),
            "Missing token should be rejected"
        );
    }

    // ========================================================================
    // TEST: RS256 Algorithm
    // ========================================================================

    // RSA test key pair (2048-bit, for testing only)
    const RSA_PRIVATE_KEY: &str = r#"-----BEGIN RSA PRIVATE KEY-----
MIIEpAIBAAKCAQEA2mKqH0dSgFzXNpHVQNB3xJ2DxYK6YQGB9lGPkXcV7VzVCwGc
FvRJHvRqYXKBHXB8j8CYXF8Xb5xMjFQqYjHZs5T4EEmVq+y0SQYeBKVrBQHB1NAJ
xLb7YvDRQDmPvVBY6YxidAFiXCYgfCnxSjLKMHX2VR1sJzJXGBMCQDpHNGFrYqKy
E5FVBcA7OjL9hJGhVvRXSqKCCjKLDZP7LfDACFAP0GCKR9zMn3TH6sOiS9xyVCqC
XCFYB8RNlVFNqHFRBdGmJ3MEfnS0VEKqhRdIQvR8/5MRr9y9mSMTQq8TjBfPB6Ot
PKWJMPqBENBsLsT/5fLMNh5mSfKqmHGqfpYKHwIDAQABAoIBAC5RgZ+hBx7xHnFZ
nQmY5lVMpGfqC0JWLNBdKDbfqCF8Q2EMl0HEGFZpNBPOBcO3WVFh0PFRYqBB5C2Z
B5lQDgLfFVKhF8ZLNkVXLmCkVYwPXLPmh9K9gFwBynLIrQ3M4eH0TZejwql+w+5L
gvZdg0k8Q0mKthZjNVqSqYkjI3pKdKKk7VOeWbCBfYfMHXJ0fPkMrF3v3DnVcuxH
HGF0sM4P4D4FqKJVLrSyZE+Q6DXkRl4qf4Oq6YZFB3E7F6O9Fj0cE4bJxPACxA8L
hBHNNHDfQMn9K8pTH0zQa0U0tkHQMdA3bXjqgJGqL9j3tQT6PMbXg0GxNQAhRP5p
dj+pT0ECgYEA7rYi8IlJ4T1FWRFWJPtGqi3pC0WVU5H0Hskq4M0u0qd1bDCqS1aK
BOHaGPVy86qq/80D9RslKkPJFnFqHgC1MBxOCv5bFPeGGWTL7BT/LRb8YnLwCm0v
mh/U0iSLzqoTt8qTCJWLPvxvDECmBq0dQ+jPMSlPqMbGhovASgmLQQ8CgYEA6XZq
MB4mhO4C4N9ovzmYJQf5raJNBk8ZfJQPvkpa3RleNLRBtScEf/sHlLZBVBkQj0cS
IkzNB8TPVQ8PfBQvLkCYVLKpqPnJurg0zhj5C0M0X7kE5BJ0HunLPFEqdH/Seoh7
hM0m6vN2zC7RTTGB+K/1dL5XZgneFYlpFqMYOIkCgYEAwUz3T3YWVJ1KcNYcC0yx
qJWMtlN6BRPvVZmMkDDIkU5qal3KdGxXB5M4pc0akxPxVxBzksyRLqTP2eMPFGqy
KVFRsFZ3Y0z/oaIB1r6m0ETN8VJ4vZGBXF+KqfBCDLsB3PCMN6gBf7vjLDGGBLwk
J4YkVLxBwMBQ5nL6f7K3bLsCgYAtX9NKsvLxepSqFuSsLnJgChBMmT/Z08vFJ1u/
jl6cI2S5hrMD+R2xBJKBiEPF9C0lK0gP5XHN1aLfwMDeJyMepkjfBP+P0Xf5f+HM
nxvDgMfKLqCr2c7Lj3FJ5q8V8LX6VQ5qhJHGnO1RDSHdN+X4aT5+M9RQPZLRJE0N
dTqKcQKBgQCJ3axJRzMA5ebKqAaRURe36XQsEYCrPl1e1LE0NnKdMNUklhYHRDCr
FXlQ8GepzSFhQkfg0rqM4F4mYBWBNjv1D+d3Ckfn8+FH9UxPD6Dz2J+JfRPdKK5M
ufHjDKbjTGA6hI1k8CDTB4vKv0VBz2hCN9eT5nyp3g6k6AZycKP6vw==
-----END RSA PRIVATE KEY-----"#;

    const RSA_PUBLIC_KEY: &str = r#"-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEA2mKqH0dSgFzXNpHVQNB3
xJ2DxYK6YQGB9lGPkXcV7VzVCwGcFvRJHvRqYXKBHXB8j8CYXF8Xb5xMjFQqYjHZ
s5T4EEmVq+y0SQYeBKVrBQHB1NAJxLb7YvDRQDmPvVBY6YxidAFiXCYgfCnxSjLK
MHX2VR1sJzJXGBMCQDpHNGFrYqKyE5FVBcA7OjL9hJGhVvRXSqKCCjKLDZP7LfDA
CFAP0GCKR9zMn3TH6sOiS9xyVCqCXCFYB8RNlVFNqHFRBdGmJ3MEfnS0VEKqhRdI
QvR8/5MRr9y9mSMTQq8TjBfPB6OtPKWJMPqBENBsLsT/5fLMNh5mSfKqmHGqfpYK
HwIDAQAB
-----END PUBLIC KEY-----"#;

    fn create_rs256_token(claims: &Claims) -> String {
        use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
        let header = Header::new(Algorithm::RS256);
        encode(
            &header,
            claims,
            &EncodingKey::from_rsa_pem(RSA_PRIVATE_KEY.as_bytes()).unwrap(),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn test_valid_rs256_token_accepted() {
        let auth = JwtAuthenticator::new_rs256(RSA_PUBLIC_KEY).unwrap();

        let claims = valid_claims();
        let token = create_rs256_token(&claims);
        let request = create_request_with_token(&token);

        let result = auth.authenticate(&request).await;
        assert!(result.is_ok(), "Valid RS256 token should be accepted");

        let auth_result = result.unwrap();
        assert_eq!(auth_result.subject, "user123");
    }

    #[tokio::test]
    async fn test_rs256_with_wrong_key_rejected() {
        // Use a different public key
        let wrong_public_key = r#"-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAu1SU1LfVLPHCozMxH2Mo
4lgOEePzNm0tRgeLezV6ffAt0gunVTLw7onLRnrq0/IzW7yWR7QkrmBL7jTKEn5u
+qKhbwKfBstIs+bMY2Zkp18gnTxKLxoS2tFczGkPLPgizskuemMghRniWaoLcyeh
kd3qqGElvW/VDL5AaWTg0nLVkjRo9z+40RQzuVaE8AkAFmxZzow3x+VJYKdjykkJ
0iT9wCS0DRTXu269V264Vf/3jvredZiKRkgwlL9xNAwxXFg0x/XFw005UWVRIkdg
cKWTjpBP2dPwVZ4WWC+9aGVd+Gyn1o0CLelf4rEjGoXbAAEgAqeGUxrcIlbjXfbc
mwIDAQAB
-----END PUBLIC KEY-----"#;

        let auth = JwtAuthenticator::new_rs256(wrong_public_key).unwrap();

        let claims = valid_claims();
        let token = create_rs256_token(&claims);
        let request = create_request_with_token(&token);

        let result = auth.authenticate(&request).await;
        assert!(
            matches!(result, Err(AuthError::InvalidSignature)),
            "RS256 token with wrong key should be rejected"
        );
    }

    // ========================================================================
    // TEST: ES256 Algorithm
    // ========================================================================

    /// Test ES256 (ECDSA with P-256) token validation
    ///
    /// RED: Will fail because ES256 support doesn't exist yet
    #[tokio::test]
    async fn test_valid_es256_token_accepted() {
        // EC P-256 test keys
        let ec_private_key = r#"-----BEGIN EC PRIVATE KEY-----
MHQCAQEEIBYr1/e/tV/9tlEPQbXbVaLsKnMFWrdkFjqwnqUMB3TpoAcGBSuBBAAK
oUQDQgAEm5vFf9scheUkkBoDF3ZxLnz6A32qfq3VYqXyCeCxPz3M7rOrVV3DDjut
Onp3OVpDBj3XvdhNiTloMAwkS+rC5A==
-----END EC PRIVATE KEY-----"#;

        let ec_public_key = r#"-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEm5vFf9scheUkkBoDF3ZxLnz6A32q
fq3VYqXyCeCxPz3M7rOrVV3DDjutOnp3OVpDBj3XvdhNiTloMAwkS+rC5A==
-----END PUBLIC KEY-----"#;

        // This should create an ES256 authenticator
        let auth = JwtAuthenticator::new_es256(ec_public_key).expect("Should create ES256 auth");

        // Create ES256 token
        use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
        let header = Header::new(Algorithm::ES256);
        let claims = valid_claims();
        let token = encode(
            &header,
            &claims,
            &EncodingKey::from_ec_pem(ec_private_key.as_bytes()).unwrap(),
        )
        .unwrap();

        let request = create_request_with_token(&token);
        let result = auth.authenticate(&request).await;

        assert!(result.is_ok(), "Valid ES256 token should be accepted");
        assert_eq!(result.unwrap().subject, "user123");
    }

    // ========================================================================
    // TEST: JWKS Endpoint Support
    // ========================================================================

    /// Test JWKS endpoint fetching and key resolution
    ///
    /// RED: Will fail because JwksAuthenticator doesn't exist yet
    #[tokio::test]
    async fn test_jwks_authenticator_fetches_keys() {
        use mizuchi_uploadr::auth::jwks::JwksAuthenticator;

        // Mock JWKS endpoint URL
        let jwks_url = "https://example.com/.well-known/jwks.json";

        // Create JWKS authenticator (should fetch keys)
        let auth = JwksAuthenticator::new(jwks_url)
            .await
            .expect("Should create JWKS authenticator");

        // The authenticator should have fetched and cached keys
        assert!(auth.has_keys(), "JWKS authenticator should have keys");
    }

    /// Test JWKS key selection by kid
    ///
    /// RED: Will fail because JwksAuthenticator doesn't exist yet
    #[tokio::test]
    async fn test_jwks_authenticator_selects_key_by_kid() {
        use mizuchi_uploadr::auth::jwks::JwksAuthenticator;

        // This test would need a mock JWKS server
        // For now, we test the key selection logic exists
        let jwks_json = r#"{
            "keys": [
                {
                    "kty": "RSA",
                    "kid": "key-1",
                    "use": "sig",
                    "alg": "RS256",
                    "n": "0vx7agoebGcQSuuPiLJXZptN9nndrQmbXEps2aiAFbWhM78LhWx4cbbfAAtVT86zwu1RK7aPFFxuhDR1L6tSoc_BJECPebWKRXjBZCiFV4n3oknjhMstn64tZ_2W-5JsGY4Hc5n9yBXArwl93lqt7_RN5w6Cf0h4QyQ5v-65YGjQR0_FDW2QvzqY368QQMicAtaSqzs8KJZgnYb9c7d0zgdAZHzu6qMQvRL5hajrn1n91CbOpbISD08qNLyrdkt-bFTWhAI4vMQFh6WeZu0fM4lFd2NcRwr3XPksINHaQ-G_xBniIqbw0Ls1jF44-csFCur-kEgU8awapJzKnqDKgw",
                    "e": "AQAB"
                },
                {
                    "kty": "RSA",
                    "kid": "key-2",
                    "use": "sig",
                    "alg": "RS256",
                    "n": "2mKqH0dSgFzXNpHVQNB3xJ2DxYK6YQGB9lGPkXcV7VzVCwGcFvRJHvRqYXKBHXB8j8CYXF8Xb5xMjFQqYjHZs5T4EEmVq-y0SQYeBKVrBQHB1NAJxLb7YvDRQDmPvVBY6YxidAFiXCYgfCnxSjLKMHX2VR1sJzJXGBMCQDpHNGFrYqKyE5FVBcA7OjL9hJGhVvRXSqKCCjKLDZP7LfDACFAP0GCKR9zMn3TH6sOiS9xyVCqCXCFYB8RNlVFNqHFRBdGmJ3MEfnS0VEKqhRdIQvR8_5MRr9y9mSMTQq8TjBfPB6OtPKWJMPqBENBsLsT_5fLMNh5mSfKqmHGqfpYKHw",
                    "e": "AQAB"
                }
            ]
        }"#;

        let auth = JwksAuthenticator::from_json(jwks_json).expect("Should parse JWKS");

        // Should be able to find key by kid
        assert!(auth.find_key("key-1").is_some(), "Should find key-1");
        assert!(auth.find_key("key-2").is_some(), "Should find key-2");
        assert!(auth.find_key("key-3").is_none(), "Should not find key-3");
    }

    /// Test JWKS key caching
    ///
    /// RED: Will fail because key caching doesn't exist yet
    #[tokio::test]
    async fn test_jwks_key_caching() {
        use mizuchi_uploadr::auth::jwks::JwksAuthenticator;
        use std::time::Duration;

        let jwks_json = r#"{"keys": []}"#;

        // Create with cache TTL
        let auth = JwksAuthenticator::from_json(jwks_json)
            .expect("Should parse JWKS")
            .with_cache_ttl(Duration::from_secs(300));

        // Cache TTL should be set
        assert_eq!(auth.cache_ttl(), Duration::from_secs(300));
    }

    /// Test JWKS-based token validation
    ///
    /// RED: Will fail because JwksAuthenticator doesn't exist yet
    #[tokio::test]
    async fn test_jwks_validates_token_with_kid() {
        use mizuchi_uploadr::auth::jwks::JwksAuthenticator;

        // JWKS with our test key
        let jwks_json = format!(
            r#"{{
            "keys": [
                {{
                    "kty": "RSA",
                    "kid": "test-key-1",
                    "use": "sig",
                    "alg": "RS256",
                    "n": "2mKqH0dSgFzXNpHVQNB3xJ2DxYK6YQGB9lGPkXcV7VzVCwGcFvRJHvRqYXKBHXB8j8CYXF8Xb5xMjFQqYjHZs5T4EEmVq-y0SQYeBKVrBQHB1NAJxLb7YvDRQDmPvVBY6YxidAFiXCYgfCnxSjLKMHX2VR1sJzJXGBMCQDpHNGFrYqKyE5FVBcA7OjL9hJGhVvRXSqKCCjKLDZP7LfDACFAP0GCKR9zMn3TH6sOiS9xyVCqCXCFYB8RNlVFNqHFRBdGmJ3MEfnS0VEKqhRdIQvR8_5MRr9y9mSMTQq8TjBfPB6OtPKWJMPqBENBsLsT_5fLMNh5mSfKqmHGqfpYKHw",
                    "e": "AQAB"
                }}
            ]
        }}"#
        );

        let auth = JwksAuthenticator::from_json(&jwks_json).expect("Should parse JWKS");

        // Create token with kid in header
        use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some("test-key-1".to_string());

        let claims = valid_claims();
        let token = encode(
            &header,
            &claims,
            &EncodingKey::from_rsa_pem(RSA_PRIVATE_KEY.as_bytes()).unwrap(),
        )
        .unwrap();

        let request = create_request_with_token(&token);
        let result = auth.authenticate(&request).await;

        assert!(result.is_ok(), "Token with valid kid should be accepted");
    }

    // ========================================================================
    // TEST: Token Extraction
    // ========================================================================

    #[tokio::test]
    async fn test_token_from_authorization_header() {
        let secret = "test-secret";
        let auth = JwtAuthenticator::new_hs256(secret);

        let claims = valid_claims();
        let token = create_hs256_token(secret, &claims);

        let mut headers = HashMap::new();
        headers.insert("authorization".to_string(), format!("Bearer {}", token));

        let request = AuthRequest {
            headers,
            query: None,
            method: "PUT".to_string(),
            path: "/bucket/key".to_string(),
        };

        let result = auth.authenticate(&request).await;
        assert!(result.is_ok(), "Should extract token from Authorization header");
    }

    #[tokio::test]
    async fn test_token_from_query_parameter() {
        let secret = "test-secret";
        let auth = JwtAuthenticator::new_hs256(secret);

        let claims = valid_claims();
        let token = create_hs256_token(secret, &claims);

        let request = AuthRequest {
            headers: HashMap::new(),
            query: Some(format!("token={}", token)),
            method: "PUT".to_string(),
            path: "/bucket/key".to_string(),
        };

        let result = auth.authenticate(&request).await;
        assert!(result.is_ok(), "Should extract token from query parameter");
    }

    // ========================================================================
    // TEST: Claims Validation
    // ========================================================================

    /// Test issuer validation
    ///
    /// RED: Will fail because issuer validation doesn't exist yet
    #[tokio::test]
    async fn test_issuer_validation() {
        let secret = "test-secret";

        // Create authenticator that requires specific issuer
        let auth = JwtAuthenticator::new_hs256(secret).with_issuer("expected-issuer");

        // Token with wrong issuer
        let mut claims = valid_claims();
        claims.iss = Some("wrong-issuer".to_string());
        let token = create_hs256_token(secret, &claims);
        let request = create_request_with_token(&token);

        let result = auth.authenticate(&request).await;
        assert!(
            matches!(result, Err(AuthError::InvalidToken(_))),
            "Token with wrong issuer should be rejected"
        );
    }

    /// Test audience validation
    ///
    /// RED: Will fail because audience validation doesn't exist yet
    #[tokio::test]
    async fn test_audience_validation() {
        let secret = "test-secret";

        // Create authenticator that requires specific audience
        let auth = JwtAuthenticator::new_hs256(secret).with_audience("expected-audience");

        // Token with wrong audience
        let mut claims = valid_claims();
        claims.aud = Some("wrong-audience".to_string());
        let token = create_hs256_token(secret, &claims);
        let request = create_request_with_token(&token);

        let result = auth.authenticate(&request).await;
        assert!(
            matches!(result, Err(AuthError::InvalidToken(_))),
            "Token with wrong audience should be rejected"
        );
    }
}
