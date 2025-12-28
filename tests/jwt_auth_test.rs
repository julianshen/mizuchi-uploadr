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
            iat: Some((chrono::Utc::now() - chrono::Duration::hours(2)).timestamp() as usize),
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

    // RSA test key pair (2048-bit, PKCS#8 format, for testing only)
    const RSA_PRIVATE_KEY: &str = r#"-----BEGIN PRIVATE KEY-----
MIIEvgIBADANBgkqhkiG9w0BAQEFAASCBKgwggSkAgEAAoIBAQCioG32/fqHZgAt
gnXnTn/VWKM+ouTdtrP8p7DdPDH894uW48oUrVFG+YICO7nUYNlJ0LBVYlLxLNbR
3VVRWAO8gKf2knMW1D4Mnmve1xPl01Wgkdcg42idaNJcQn1cHygvJJo0LE0/5l3l
1AuvV9Kjg/ibDMYxMwqm0KRlk4kHvcJW3AT7urp6faxszBkE0F//NxXSYdxocru0
cyDmzeloYN8xwSIWxQoBNVKQNDmT7NnKs6P1TUB2pUS6mUqZY65C3iXu3iefbaKz
d7bjbwb8py+RC7L7t7jSg7oq663FPTwycNEVFYemyJU7WCqGjIxkQrpLdb9rLOgN
8+OQsd+fAgMBAAECggEAD21aoSicwIVrlOWgW631sH51FfcL8QBo+JnLzGDueQne
oxS+0dFTOYFn1OBnk38QfdEfOSpXpetUAZqWgl3wFMy7okdoRY0iyb2pi/0pNQ8k
O1Q6bTNFdFFCS2A/ViLahAZb3oEpXttyot/Hr/2LzNkzFzpR/s7RvttiDQS+5g3y
WhJmyvgD7ui5HWpRP2dphonJt9JgbBHDYdHjc0G2mkCdQ1ZIOopV4N2nqe/4Q4tB
mLboyLdBJm0Z5VbUeojDj6L/HUDTbT2QsZ9IsD+iodIt+8CcnpTGeZ2f3PYJ5Km1
5ddGb6qru0rthTC2SK4LBfkIq8MqG3bmnnbG27iAUQKBgQDiiL92/4j4lioIuM3a
sxzvUVnPnf2eWkuqrMcXoab75fLDxEZIpUjbOyCAhD0qshqUlPIdCgdUbZBOGbME
GvV8ut0Tal5j1qE92AkALj5FITHSAW6nbCrXDViR1r/9k+0IHAaJbNM3+1pz46d+
NrB1dHe2B5g46qgTOOKJ3RcFuQKBgQC3x6gm0gldyu807h4reKs1I37NfRDZHG78
IQayoHoSHhzs26XwWGKDDj5ONqPUjcHAnrJFVr6eWMqAWU2jYAgRzuOEZ24nblT7
b2zOhw70nDe7Hm7zyx4nTYC+4/dbldf7xnuQIcTo2eRwz2Ig0/GDmm4Nf0qHAu1r
J/EN1RE8FwKBgQCaqVJPJFeXoK5CFio1TmRK3/e5T9x/6JYQiLXE5JDlGjGMhsyV
fIMpakzecWpxY/fRyX8jZF1svwDu0YzvGJjR96JIRy76aubbGkvK28eX2vnwrxml
JKx69pmpuDyMHBqQltG/sZTje7Bdvufzu9Lt3f59QOIkudDWjtfb2B6HwQKBgC6Z
ACi/rsJKVzabfajWEssJcfhWUrRKAlYJZbJbADihzAG+e6eiMXA7Z07bidS2EL9v
PZJZOUHbD5VVj1ryWXlydLu4ofR7hC6whO0kz4T0KylVwRotkTqz6wX7tVdSeg4L
uH7GITBNNx/nZWEffCg7OtZPRS1Qb7Rwzy0LrjAHAoGBALwuyWRGY+tdrbQwGwFd
ODUjHpkVlAH6siEi4u734RorwYa8h4WsZjUInr7m9fp55Wfe/3VBUgnM6YkrecVk
1EaTn7UHAlT+kQ2c5BQ13n9kc6iZP/Mz/aXm/ZTEpJL8OqOSrEJYZck2bqKwZ19S
CiKoEJ3Dnq+RVO82q/iriqT8
-----END PRIVATE KEY-----"#;

    const RSA_PUBLIC_KEY: &str = r#"-----BEGIN PUBLIC KEY-----
MIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAoqBt9v36h2YALYJ1505/
1VijPqLk3baz/Kew3Twx/PeLluPKFK1RRvmCAju51GDZSdCwVWJS8SzW0d1VUVgD
vICn9pJzFtQ+DJ5r3tcT5dNVoJHXIONonWjSXEJ9XB8oLySaNCxNP+Zd5dQLr1fS
o4P4mwzGMTMKptCkZZOJB73CVtwE+7q6en2sbMwZBNBf/zcV0mHcaHK7tHMg5s3p
aGDfMcEiFsUKATVSkDQ5k+zZyrOj9U1AdqVEuplKmWOuQt4l7t4nn22is3e2428G
/KcvkQuy+7e40oO6KuutxT08MnDRFRWHpsiVO1gqhoyMZEK6S3W/ayzoDfPjkLHf
nwIDAQAB
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
    /// GREEN: ES256 support added via new_es256()
    #[tokio::test]
    async fn test_valid_es256_token_accepted() {
        // EC P-256 (secp256r1) test keys in PKCS#8 format
        let ec_private_key = r#"-----BEGIN PRIVATE KEY-----
MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQg3dpHBGw+qKVN15qn
hPjY/Lh9SUooPZx13tcRdi+Gj++hRANCAARtMxM0KEkKrYTU2aQg81IOFFl5Lvs/
aqlY8ML+o0rzdQ671jbpVjurBUfgzoPyM9ek3yNIbCAJ1UKkjloSJiBu
-----END PRIVATE KEY-----"#;

        let ec_public_key = r#"-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEbTMTNChJCq2E1NmkIPNSDhRZeS77
P2qpWPDC/qNK83UOu9Y26VY7qwVH4M6D8jPXpN8jSGwgCdVCpI5aEiYgbg==
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
    /// This test requires a real JWKS endpoint, so it's ignored by default.
    /// Run with: cargo test -- --ignored test_jwks_authenticator_fetches_keys
    #[tokio::test]
    #[ignore = "Requires real JWKS endpoint"]
    async fn test_jwks_authenticator_fetches_keys() {
        use mizuchi_uploadr::auth::jwks::JwksAuthenticator;

        // Mock JWKS endpoint URL
        let jwks_url = "https://example.com/.well-known/jwks.json";

        // Create JWKS authenticator (should fetch keys)
        let auth = JwksAuthenticator::new(jwks_url)
            .await
            .expect("Should create JWKS authenticator");

        // The authenticator should have fetched and cached keys
        assert!(auth.has_keys().await, "JWKS authenticator should have keys");
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
        assert!(auth.find_key("key-1").await.is_some(), "Should find key-1");
        assert!(auth.find_key("key-2").await.is_some(), "Should find key-2");
        assert!(
            auth.find_key("key-3").await.is_none(),
            "Should not find key-3"
        );
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
    /// GREEN: JwksAuthenticator implemented
    #[tokio::test]
    async fn test_jwks_validates_token_with_kid() {
        use mizuchi_uploadr::auth::jwks::JwksAuthenticator;

        // JWKS with our test key (modulus matches RSA_PUBLIC_KEY)
        let jwks_json = r#"{
            "keys": [
                {
                    "kty": "RSA",
                    "kid": "test-key-1",
                    "use": "sig",
                    "alg": "RS256",
                    "n": "oqBt9v36h2YALYJ1505_1VijPqLk3baz_Kew3Twx_PeLluPKFK1RRvmCAju51GDZSdCwVWJS8SzW0d1VUVgDvICn9pJzFtQ-DJ5r3tcT5dNVoJHXIONonWjSXEJ9XB8oLySaNCxNP-Zd5dQLr1fSo4P4mwzGMTMKptCkZZOJB73CVtwE-7q6en2sbMwZBNBf_zcV0mHcaHK7tHMg5s3paGDfMcEiFsUKATVSkDQ5k-zZyrOj9U1AdqVEuplKmWOuQt4l7t4nn22is3e2428G_KcvkQuy-7e40oO6KuutxT08MnDRFRWHpsiVO1gqhoyMZEK6S3W_ayzoDfPjkLHfnw",
                    "e": "AQAB"
                }
            ]
        }"#;

        let auth = JwksAuthenticator::from_json(jwks_json).expect("Should parse JWKS");

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
        assert!(
            result.is_ok(),
            "Should extract token from Authorization header"
        );
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

    // ========================================================================
    // TEST: JWKS Issuer/Audience Validation (Refactor phase)
    // ========================================================================

    #[tokio::test]
    async fn test_jwks_with_issuer_validation() {
        use mizuchi_uploadr::auth::jwks::JwksAuthenticator;

        let jwks_json = r#"{
            "keys": [
                {
                    "kty": "RSA",
                    "kid": "test-key-1",
                    "use": "sig",
                    "alg": "RS256",
                    "n": "oqBt9v36h2YALYJ1505_1VijPqLk3baz_Kew3Twx_PeLluPKFK1RRvmCAju51GDZSdCwVWJS8SzW0d1VUVgDvICn9pJzFtQ-DJ5r3tcT5dNVoJHXIONonWjSXEJ9XB8oLySaNCxNP-Zd5dQLr1fSo4P4mwzGMTMKptCkZZOJB73CVtwE-7q6en2sbMwZBNBf_zcV0mHcaHK7tHMg5s3paGDfMcEiFsUKATVSkDQ5k-zZyrOj9U1AdqVEuplKmWOuQt4l7t4nn22is3e2428G_KcvkQuy-7e40oO6KuutxT08MnDRFRWHpsiVO1gqhoyMZEK6S3W_ayzoDfPjkLHfnw",
                    "e": "AQAB"
                }
            ]
        }"#;

        // Configure JWKS with required issuer
        let auth = JwksAuthenticator::from_json(jwks_json)
            .expect("Should parse JWKS")
            .with_issuer("expected-issuer");

        // Create token with wrong issuer
        use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some("test-key-1".to_string());

        let mut claims = valid_claims();
        claims.iss = Some("wrong-issuer".to_string());
        let token = encode(
            &header,
            &claims,
            &EncodingKey::from_rsa_pem(RSA_PRIVATE_KEY.as_bytes()).unwrap(),
        )
        .unwrap();

        let request = create_request_with_token(&token);
        let result = auth.authenticate(&request).await;

        assert!(
            matches!(result, Err(AuthError::InvalidToken(_))),
            "JWKS should reject token with wrong issuer"
        );
    }

    #[tokio::test]
    async fn test_jwks_with_audience_validation() {
        use mizuchi_uploadr::auth::jwks::JwksAuthenticator;

        let jwks_json = r#"{
            "keys": [
                {
                    "kty": "RSA",
                    "kid": "test-key-1",
                    "use": "sig",
                    "alg": "RS256",
                    "n": "oqBt9v36h2YALYJ1505_1VijPqLk3baz_Kew3Twx_PeLluPKFK1RRvmCAju51GDZSdCwVWJS8SzW0d1VUVgDvICn9pJzFtQ-DJ5r3tcT5dNVoJHXIONonWjSXEJ9XB8oLySaNCxNP-Zd5dQLr1fSo4P4mwzGMTMKptCkZZOJB73CVtwE-7q6en2sbMwZBNBf_zcV0mHcaHK7tHMg5s3paGDfMcEiFsUKATVSkDQ5k-zZyrOj9U1AdqVEuplKmWOuQt4l7t4nn22is3e2428G_KcvkQuy-7e40oO6KuutxT08MnDRFRWHpsiVO1gqhoyMZEK6S3W_ayzoDfPjkLHfnw",
                    "e": "AQAB"
                }
            ]
        }"#;

        // Configure JWKS with required audience
        let auth = JwksAuthenticator::from_json(jwks_json)
            .expect("Should parse JWKS")
            .with_audience("expected-audience");

        // Create token with wrong audience
        use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some("test-key-1".to_string());

        let mut claims = valid_claims();
        claims.aud = Some("wrong-audience".to_string());
        let token = encode(
            &header,
            &claims,
            &EncodingKey::from_rsa_pem(RSA_PRIVATE_KEY.as_bytes()).unwrap(),
        )
        .unwrap();

        let request = create_request_with_token(&token);
        let result = auth.authenticate(&request).await;

        assert!(
            matches!(result, Err(AuthError::InvalidToken(_))),
            "JWKS should reject token with wrong audience"
        );
    }
}
