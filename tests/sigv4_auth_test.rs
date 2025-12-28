//! AWS SigV4 Authentication Integration Tests
//!
//! TDD RED Phase: These tests define expected SigV4 authentication behavior.
//! Tests cover signature validation, timestamp verification, and replay prevention.

#[cfg(test)]
mod tests {
    use mizuchi_uploadr::auth::sigv4::SigV4Authenticator;
    use mizuchi_uploadr::auth::{AuthError, AuthRequest, Authenticator};
    use std::collections::HashMap;

    // ========================================================================
    // Helper: Test credentials and utilities
    // ========================================================================

    /// Test credentials (from AWS documentation examples)
    const TEST_ACCESS_KEY: &str = "AKIAIOSFODNN7EXAMPLE";
    const TEST_SECRET_KEY: &str = "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY";
    const TEST_REGION: &str = "us-east-1";
    const TEST_SERVICE: &str = "s3";

    fn current_timestamp() -> String {
        chrono::Utc::now().format("%Y%m%dT%H%M%SZ").to_string()
    }

    fn current_date() -> String {
        chrono::Utc::now().format("%Y%m%d").to_string()
    }

    /// Create a request with manually constructed authorization header
    fn create_request_with_auth(
        method: &str,
        path: &str,
        authorization: &str,
        x_amz_date: &str,
        content_sha256: Option<&str>,
    ) -> AuthRequest {
        let mut headers = HashMap::new();
        headers.insert("authorization".to_string(), authorization.to_string());
        headers.insert("host".to_string(), "s3.us-east-1.amazonaws.com".to_string());
        headers.insert("x-amz-date".to_string(), x_amz_date.to_string());
        if let Some(hash) = content_sha256 {
            headers.insert("x-amz-content-sha256".to_string(), hash.to_string());
        } else {
            // Empty body hash
            headers.insert(
                "x-amz-content-sha256".to_string(),
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
            );
        }

        AuthRequest {
            headers,
            query: None,
            method: method.to_string(),
            path: path.to_string(),
        }
    }

    /// Compute HMAC-SHA256
    fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;

        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(key).expect("HMAC can take key of any size");
        mac.update(data);
        mac.finalize().into_bytes().to_vec()
    }

    /// Compute SHA256 hash
    fn sha256_hex(data: &[u8]) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }

    /// Generate a valid SigV4 signature for testing
    /// This follows the AWS SigV4 signing process exactly
    fn generate_signature(
        secret_key: &str,
        date: &str,
        region: &str,
        service: &str,
        string_to_sign: &str,
    ) -> String {
        // Step 1: Create signing key
        // kSecret = "AWS4" + secret_key
        // kDate = HMAC("AWS4" + secret_key, date)
        // kRegion = HMAC(kDate, region)
        // kService = HMAC(kRegion, service)
        // kSigning = HMAC(kService, "aws4_request")
        let k_secret = format!("AWS4{}", secret_key);
        let k_date = hmac_sha256(k_secret.as_bytes(), date.as_bytes());
        let k_region = hmac_sha256(&k_date, region.as_bytes());
        let k_service = hmac_sha256(&k_region, service.as_bytes());
        let k_signing = hmac_sha256(&k_service, b"aws4_request");

        // Step 2: Calculate signature
        let signature = hmac_sha256(&k_signing, string_to_sign.as_bytes());
        hex::encode(signature)
    }

    /// Create a properly signed request
    fn create_valid_signed_request(method: &str, path: &str, body: &[u8]) -> AuthRequest {
        let timestamp = current_timestamp();
        let date = current_date();
        let content_hash = sha256_hex(body);
        let host = "s3.us-east-1.amazonaws.com";

        // Canonical request
        let canonical_headers = format!(
            "host:{}\nx-amz-content-sha256:{}\nx-amz-date:{}\n",
            host, content_hash, timestamp
        );
        let signed_headers = "host;x-amz-content-sha256;x-amz-date";

        let canonical_request = format!(
            "{}\n{}\n\n{}\n{}\n{}",
            method, path, canonical_headers, signed_headers, content_hash
        );

        let canonical_request_hash = sha256_hex(canonical_request.as_bytes());

        // String to sign
        let credential_scope = format!("{}/{}/{}/aws4_request", date, TEST_REGION, TEST_SERVICE);
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{}\n{}\n{}",
            timestamp, credential_scope, canonical_request_hash
        );

        // Generate signature
        let signature = generate_signature(
            TEST_SECRET_KEY,
            &date,
            TEST_REGION,
            TEST_SERVICE,
            &string_to_sign,
        );

        // Build authorization header
        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            TEST_ACCESS_KEY, credential_scope, signed_headers, signature
        );

        create_request_with_auth(
            method,
            path,
            &authorization,
            &timestamp,
            Some(&content_hash),
        )
    }

    // ========================================================================
    // TEST: Valid Signature Accepted
    // ========================================================================

    /// Test that a properly signed request is accepted
    ///
    /// RED: Will fail because signature validation isn't implemented
    #[tokio::test]
    async fn test_valid_signature_accepted() {
        let mut auth = SigV4Authenticator::new(TEST_SERVICE, TEST_REGION);
        auth.add_credentials(TEST_ACCESS_KEY, TEST_SECRET_KEY);

        let request = create_valid_signed_request("PUT", "/bucket/key", b"test data");

        let result = auth.authenticate(&request).await;
        assert!(
            result.is_ok(),
            "Valid signature should be accepted: {:?}",
            result
        );

        let auth_result = result.unwrap();
        assert_eq!(auth_result.subject, TEST_ACCESS_KEY);
    }

    /// Test empty body request is valid
    ///
    /// RED: Will fail because signature validation isn't implemented
    #[tokio::test]
    async fn test_valid_empty_body_signature() {
        let mut auth = SigV4Authenticator::new(TEST_SERVICE, TEST_REGION);
        auth.add_credentials(TEST_ACCESS_KEY, TEST_SECRET_KEY);

        let request = create_valid_signed_request("PUT", "/bucket/key", b"");

        let result = auth.authenticate(&request).await;
        assert!(
            result.is_ok(),
            "Valid empty body signature should be accepted: {:?}",
            result
        );
    }

    // ========================================================================
    // TEST: Invalid Signature Rejected
    // ========================================================================

    /// Test that a request with an invalid signature is rejected
    ///
    /// RED: Will fail because signature validation isn't implemented
    #[tokio::test]
    async fn test_invalid_signature_rejected() {
        let mut auth = SigV4Authenticator::new(TEST_SERVICE, TEST_REGION);
        auth.add_credentials(TEST_ACCESS_KEY, TEST_SECRET_KEY);

        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={}/{}/{}/{}/aws4_request, \
             SignedHeaders=host;x-amz-content-sha256;x-amz-date, \
             Signature=0000000000000000000000000000000000000000000000000000000000000000",
            TEST_ACCESS_KEY,
            current_date(),
            TEST_REGION,
            TEST_SERVICE
        );

        let request = create_request_with_auth(
            "PUT",
            "/bucket/key",
            &authorization,
            &current_timestamp(),
            None,
        );

        let result = auth.authenticate(&request).await;
        assert!(
            matches!(result, Err(AuthError::InvalidSignature)),
            "Invalid signature should be rejected: {:?}",
            result
        );
    }

    /// Test that a request signed with wrong secret key is rejected
    ///
    /// RED: Will fail because signature validation isn't implemented
    #[tokio::test]
    async fn test_wrong_secret_key_rejected() {
        let mut auth = SigV4Authenticator::new(TEST_SERVICE, TEST_REGION);
        // Register with different secret key than what was used to sign
        auth.add_credentials(TEST_ACCESS_KEY, "wrong-secret-key-12345");

        // This request is signed with TEST_SECRET_KEY
        let request = create_valid_signed_request("PUT", "/bucket/key", b"test data");

        let result = auth.authenticate(&request).await;
        assert!(
            matches!(result, Err(AuthError::InvalidSignature)),
            "Request signed with wrong secret should be rejected: {:?}",
            result
        );
    }

    // ========================================================================
    // TEST: Timestamp Validation (Replay Attack Prevention)
    // ========================================================================

    /// Test that requests with expired timestamps are rejected
    ///
    /// RED: Will fail because timestamp validation isn't implemented
    #[tokio::test]
    async fn test_expired_timestamp_rejected() {
        let mut auth = SigV4Authenticator::new(TEST_SERVICE, TEST_REGION);
        auth.add_credentials(TEST_ACCESS_KEY, TEST_SECRET_KEY);

        // Use timestamp from 20 minutes ago (AWS allows 15 min skew)
        let old_time = chrono::Utc::now() - chrono::Duration::minutes(20);
        let old_timestamp = old_time.format("%Y%m%dT%H%M%SZ").to_string();
        let old_date = old_time.format("%Y%m%d").to_string();

        // Build a request with old timestamp but valid signature for that timestamp
        let content_hash = sha256_hex(b"");
        let host = "s3.us-east-1.amazonaws.com";
        let canonical_headers = format!(
            "host:{}\nx-amz-content-sha256:{}\nx-amz-date:{}\n",
            host, content_hash, old_timestamp
        );
        let signed_headers = "host;x-amz-content-sha256;x-amz-date";
        let canonical_request = format!(
            "PUT\n/bucket/key\n\n{}\n{}\n{}",
            canonical_headers, signed_headers, content_hash
        );
        let canonical_request_hash = sha256_hex(canonical_request.as_bytes());
        let credential_scope =
            format!("{}/{}/{}/aws4_request", old_date, TEST_REGION, TEST_SERVICE);
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{}\n{}\n{}",
            old_timestamp, credential_scope, canonical_request_hash
        );
        let signature = generate_signature(
            TEST_SECRET_KEY,
            &old_date,
            TEST_REGION,
            TEST_SERVICE,
            &string_to_sign,
        );
        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            TEST_ACCESS_KEY, credential_scope, signed_headers, signature
        );

        let request = create_request_with_auth(
            "PUT",
            "/bucket/key",
            &authorization,
            &old_timestamp,
            Some(&content_hash),
        );

        let result = auth.authenticate(&request).await;
        assert!(
            matches!(
                result,
                Err(AuthError::InvalidToken(_)) | Err(AuthError::TokenExpired)
            ),
            "Expired timestamp should be rejected: {:?}",
            result
        );
    }

    /// Test that requests with future timestamps are rejected
    ///
    /// RED: Will fail because timestamp validation isn't implemented
    #[tokio::test]
    async fn test_future_timestamp_rejected() {
        let mut auth = SigV4Authenticator::new(TEST_SERVICE, TEST_REGION);
        auth.add_credentials(TEST_ACCESS_KEY, TEST_SECRET_KEY);

        // Use timestamp from 20 minutes in the future
        let future_time = chrono::Utc::now() + chrono::Duration::minutes(20);
        let future_timestamp = future_time.format("%Y%m%dT%H%M%SZ").to_string();
        let future_date = future_time.format("%Y%m%d").to_string();

        // Build a request with future timestamp
        let content_hash = sha256_hex(b"");
        let host = "s3.us-east-1.amazonaws.com";
        let canonical_headers = format!(
            "host:{}\nx-amz-content-sha256:{}\nx-amz-date:{}\n",
            host, content_hash, future_timestamp
        );
        let signed_headers = "host;x-amz-content-sha256;x-amz-date";
        let canonical_request = format!(
            "PUT\n/bucket/key\n\n{}\n{}\n{}",
            canonical_headers, signed_headers, content_hash
        );
        let canonical_request_hash = sha256_hex(canonical_request.as_bytes());
        let credential_scope = format!(
            "{}/{}/{}/aws4_request",
            future_date, TEST_REGION, TEST_SERVICE
        );
        let string_to_sign = format!(
            "AWS4-HMAC-SHA256\n{}\n{}\n{}",
            future_timestamp, credential_scope, canonical_request_hash
        );
        let signature = generate_signature(
            TEST_SECRET_KEY,
            &future_date,
            TEST_REGION,
            TEST_SERVICE,
            &string_to_sign,
        );
        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={}/{}, SignedHeaders={}, Signature={}",
            TEST_ACCESS_KEY, credential_scope, signed_headers, signature
        );

        let request = create_request_with_auth(
            "PUT",
            "/bucket/key",
            &authorization,
            &future_timestamp,
            Some(&content_hash),
        );

        let result = auth.authenticate(&request).await;
        assert!(
            matches!(result, Err(AuthError::InvalidToken(_))),
            "Future timestamp should be rejected: {:?}",
            result
        );
    }

    // ========================================================================
    // TEST: Missing Required Headers
    // ========================================================================

    /// Test that missing x-amz-date header is rejected
    ///
    /// RED: Will fail because header validation isn't fully implemented
    #[tokio::test]
    async fn test_missing_date_header_rejected() {
        let mut auth = SigV4Authenticator::new(TEST_SERVICE, TEST_REGION);
        auth.add_credentials(TEST_ACCESS_KEY, TEST_SECRET_KEY);

        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential={}/{}/{}/{}/aws4_request, \
             SignedHeaders=host;x-amz-content-sha256, \
             Signature=1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            TEST_ACCESS_KEY,
            current_date(),
            TEST_REGION,
            TEST_SERVICE
        );

        let mut headers = HashMap::new();
        headers.insert("authorization".to_string(), authorization);
        headers.insert("host".to_string(), "s3.us-east-1.amazonaws.com".to_string());
        headers.insert(
            "x-amz-content-sha256".to_string(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
        );
        // Missing x-amz-date!

        let request = AuthRequest {
            headers,
            query: None,
            method: "PUT".to_string(),
            path: "/bucket/key".to_string(),
        };

        let result = auth.authenticate(&request).await;
        assert!(
            matches!(result, Err(AuthError::InvalidToken(_))),
            "Missing x-amz-date should be rejected: {:?}",
            result
        );
    }

    // ========================================================================
    // TEST: Unknown Access Key
    // ========================================================================

    /// Test that unknown access key is rejected
    ///
    /// RED: Will fail because credential lookup isn't properly implemented
    #[tokio::test]
    async fn test_unknown_access_key_rejected() {
        let auth = SigV4Authenticator::new(TEST_SERVICE, TEST_REGION);
        // Don't add any credentials

        let authorization = format!(
            "AWS4-HMAC-SHA256 Credential=UNKNOWN_KEY/{}/{}/{}/aws4_request, \
             SignedHeaders=host;x-amz-content-sha256;x-amz-date, \
             Signature=1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            current_date(),
            TEST_REGION,
            TEST_SERVICE
        );

        let request = create_request_with_auth(
            "PUT",
            "/bucket/key",
            &authorization,
            &current_timestamp(),
            None,
        );

        let result = auth.authenticate(&request).await;
        assert!(
            matches!(result, Err(AuthError::InvalidToken(_))),
            "Unknown access key should be rejected: {:?}",
            result
        );
    }

    // ========================================================================
    // TEST: Invalid Authorization Header Format
    // ========================================================================

    /// Test that non-SigV4 authorization is rejected
    #[tokio::test]
    async fn test_non_sigv4_auth_rejected() {
        let auth = SigV4Authenticator::new(TEST_SERVICE, TEST_REGION);

        let mut headers = HashMap::new();
        headers.insert(
            "authorization".to_string(),
            "Basic dXNlcjpwYXNz".to_string(),
        );
        headers.insert("host".to_string(), "s3.us-east-1.amazonaws.com".to_string());
        headers.insert("x-amz-date".to_string(), current_timestamp());

        let request = AuthRequest {
            headers,
            query: None,
            method: "PUT".to_string(),
            path: "/bucket/key".to_string(),
        };

        let result = auth.authenticate(&request).await;
        assert!(
            matches!(result, Err(AuthError::InvalidToken(_))),
            "Non-SigV4 auth should be rejected"
        );
    }

    /// Test that malformed Credential is rejected
    #[tokio::test]
    async fn test_malformed_credential_rejected() {
        let auth = SigV4Authenticator::new(TEST_SERVICE, TEST_REGION);

        let authorization = "AWS4-HMAC-SHA256 Credential=INVALID_FORMAT, \
             SignedHeaders=host, Signature=abc123";

        let request = create_request_with_auth(
            "PUT",
            "/bucket/key",
            authorization,
            &current_timestamp(),
            None,
        );

        let result = auth.authenticate(&request).await;
        assert!(
            matches!(result, Err(AuthError::InvalidToken(_))),
            "Malformed credential should be rejected"
        );
    }

    // ========================================================================
    // TEST: Request Tampering Detection
    // ========================================================================

    /// Test that signature covers the request method
    ///
    /// RED: Will fail because full signature validation isn't implemented
    #[tokio::test]
    async fn test_method_tampering_rejected() {
        let mut auth = SigV4Authenticator::new(TEST_SERVICE, TEST_REGION);
        auth.add_credentials(TEST_ACCESS_KEY, TEST_SECRET_KEY);

        // Sign as PUT but change to DELETE
        let mut request = create_valid_signed_request("PUT", "/bucket/key", b"");
        request.method = "DELETE".to_string();

        let result = auth.authenticate(&request).await;
        assert!(
            matches!(result, Err(AuthError::InvalidSignature)),
            "Method tampering should be rejected: {:?}",
            result
        );
    }

    /// Test that signature covers the request path
    ///
    /// RED: Will fail because full signature validation isn't implemented
    #[tokio::test]
    async fn test_path_tampering_rejected() {
        let mut auth = SigV4Authenticator::new(TEST_SERVICE, TEST_REGION);
        auth.add_credentials(TEST_ACCESS_KEY, TEST_SECRET_KEY);

        // Sign for /bucket/key but change to /bucket/other
        let mut request = create_valid_signed_request("PUT", "/bucket/key", b"");
        request.path = "/bucket/other".to_string();

        let result = auth.authenticate(&request).await;
        assert!(
            matches!(result, Err(AuthError::InvalidSignature)),
            "Path tampering should be rejected: {:?}",
            result
        );
    }
}
