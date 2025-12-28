//! E2E Authentication Flow Tests
//!
//! RED Phase: These tests define the expected behavior for authentication
//! integrated with the upload flow.
//!
//! ## Test Coverage
//!
//! - JWT authentication with valid tokens
//! - JWT authentication with expired tokens
//! - JWT authentication with invalid tokens
//! - SigV4 authentication with valid signatures
//! - SigV4 authentication with invalid signatures
//! - Unauthenticated requests when auth is required
//!
//! ## Note on Auth Enforcement Tests
//!
//! Tests marked with `#[ignore]` require the server to enforce JWT authentication.
//! Currently, the server stub in `pingora.rs` doesn't enforce auth.
//! These tests document expected behavior and will pass once auth is implemented.

use super::common::E2ETestEnv;
use std::time::Duration;

/// Test: Upload with valid JWT token succeeds
#[tokio::test]
async fn test_upload_with_valid_jwt_succeeds() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let config = E2ETestEnv::config_with_jwt(0);
    let env = E2ETestEnv::with_config(config)
        .await
        .expect("Failed to create test env");

    // Generate valid token
    let token = E2ETestEnv::generate_test_jwt("test-user", 3600);

    let response = env
        .put_object_with_jwt("/uploads/auth-test.txt", b"authenticated content", &token)
        .await
        .expect("Request failed");

    assert!(
        response.status().is_success(),
        "Upload with valid JWT should succeed, got: {}",
        response.status()
    );
}

/// Test: Upload with expired JWT token fails with 401
///
/// RED Phase: This test documents expected behavior when auth is enforced.
/// Currently ignored because the server stub doesn't enforce JWT validation.
#[tokio::test]
#[ignore = "Server auth enforcement not yet implemented - RED phase test"]
async fn test_upload_with_expired_jwt_returns_401() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let config = E2ETestEnv::config_with_jwt(0);
    let env = E2ETestEnv::with_config(config)
        .await
        .expect("Failed to create test env");

    // Generate expired token
    let token = E2ETestEnv::generate_expired_jwt("test-user");

    let response = env
        .put_object_with_jwt("/uploads/auth-test.txt", b"should fail", &token)
        .await
        .expect("Request failed");

    assert_eq!(
        response.status().as_u16(),
        401,
        "Upload with expired JWT should return 401, got: {}",
        response.status()
    );
}

/// Test: Upload with invalid JWT signature fails with 401
///
/// RED Phase: This test documents expected behavior when auth is enforced.
#[tokio::test]
#[ignore = "Server auth enforcement not yet implemented - RED phase test"]
async fn test_upload_with_invalid_jwt_signature_returns_401() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let config = E2ETestEnv::config_with_jwt(0);
    let env = E2ETestEnv::with_config(config)
        .await
        .expect("Failed to create test env");

    // Create a token with wrong secret (tampered signature)
    let invalid_token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.\
                         eyJzdWIiOiJ0ZXN0LXVzZXIiLCJleHAiOjk5OTk5OTk5OTl9.\
                         INVALID_SIGNATURE_THAT_WONT_VERIFY";

    let response = env
        .put_object_with_jwt("/uploads/auth-test.txt", b"should fail", invalid_token)
        .await
        .expect("Request failed");

    assert_eq!(
        response.status().as_u16(),
        401,
        "Upload with invalid JWT signature should return 401, got: {}",
        response.status()
    );
}

/// Test: Upload with malformed JWT fails with 401
///
/// RED Phase: This test documents expected behavior when auth is enforced.
#[tokio::test]
#[ignore = "Server auth enforcement not yet implemented - RED phase test"]
async fn test_upload_with_malformed_jwt_returns_401() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let config = E2ETestEnv::config_with_jwt(0);
    let env = E2ETestEnv::with_config(config)
        .await
        .expect("Failed to create test env");

    // Malformed token (not base64)
    let malformed_token = "not-a-valid-jwt-token";

    let response = env
        .put_object_with_jwt("/uploads/auth-test.txt", b"should fail", malformed_token)
        .await
        .expect("Request failed");

    assert_eq!(
        response.status().as_u16(),
        401,
        "Upload with malformed JWT should return 401, got: {}",
        response.status()
    );
}

/// Test: Upload without JWT when required fails with 401
///
/// RED Phase: This test documents expected behavior when auth is enforced.
#[tokio::test]
#[ignore = "Server auth enforcement not yet implemented - RED phase test"]
async fn test_upload_without_jwt_when_required_returns_401() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let config = E2ETestEnv::config_with_jwt(0);
    let env = E2ETestEnv::with_config(config)
        .await
        .expect("Failed to create test env");

    // No Authorization header
    let response = env
        .put_object("/uploads/auth-test.txt", b"should fail", Some("text/plain"))
        .await
        .expect("Request failed");

    assert_eq!(
        response.status().as_u16(),
        401,
        "Upload without JWT should return 401, got: {}",
        response.status()
    );
}

/// Test: Upload with wrong Authorization scheme fails with 401
///
/// RED Phase: This test documents expected behavior when auth is enforced.
#[tokio::test]
#[ignore = "Server auth enforcement not yet implemented - RED phase test"]
async fn test_upload_with_wrong_auth_scheme_returns_401() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let config = E2ETestEnv::config_with_jwt(0);
    let env = E2ETestEnv::with_config(config)
        .await
        .expect("Failed to create test env");

    // Use Basic auth instead of Bearer
    let response = env
        .client
        .put(format!("{}/uploads/auth-test.txt", env.base_url()))
        .header("Authorization", "Basic dXNlcjpwYXNz") // Base64 of "user:pass"
        .body("should fail")
        .send()
        .await
        .expect("Request failed");

    assert_eq!(
        response.status().as_u16(),
        401,
        "Upload with wrong auth scheme should return 401, got: {}",
        response.status()
    );
}

/// Test: Multiple authenticated uploads from same user
#[tokio::test]
async fn test_multiple_authenticated_uploads() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let config = E2ETestEnv::config_with_jwt(0);
    let env = E2ETestEnv::with_config(config)
        .await
        .expect("Failed to create test env");

    let token = E2ETestEnv::generate_test_jwt("multi-upload-user", 3600);

    // Upload multiple files with same token
    for i in 0..5 {
        let response = env
            .put_object_with_jwt(
                &format!("/uploads/multi-auth-{}.txt", i),
                format!("content {}", i).as_bytes(),
                &token,
            )
            .await
            .expect("Request failed");

        assert!(
            response.status().is_success(),
            "Upload {} with valid JWT should succeed",
            i
        );
    }
}

/// Test: Concurrent authenticated uploads
#[tokio::test]
async fn test_concurrent_authenticated_uploads() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let config = E2ETestEnv::config_with_jwt(0);
    let env = E2ETestEnv::with_config(config)
        .await
        .expect("Failed to create test env");

    let token = E2ETestEnv::generate_test_jwt("concurrent-user", 3600);

    // Launch 10 concurrent authenticated uploads
    let mut handles = vec![];
    for i in 0..10 {
        let client = env.client.clone();
        let base_url = env.base_url();
        let token = token.clone();
        let payload = E2ETestEnv::random_payload(10 * 1024); // 10KB each

        let handle = tokio::spawn(async move {
            let response = client
                .put(format!("{}/uploads/concurrent-auth-{}.bin", base_url, i))
                .header("Authorization", format!("Bearer {}", token))
                .body(payload.to_vec())
                .send()
                .await;

            response.map(|r| r.status().is_success())
        });

        handles.push(handle);
    }

    // Wait for all uploads to complete
    let results: Vec<_> = futures::future::join_all(handles).await;

    // All uploads should succeed
    for (i, result) in results.iter().enumerate() {
        match result {
            Ok(Ok(true)) => {}
            Ok(Ok(false)) => panic!("Concurrent auth upload {} failed", i),
            Ok(Err(e)) => panic!("Concurrent auth upload {} error: {}", i, e),
            Err(e) => panic!("Concurrent auth upload {} panicked: {}", i, e),
        }
    }
}

/// Test: Token with different subjects can upload to same bucket
#[tokio::test]
async fn test_different_users_can_upload() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let config = E2ETestEnv::config_with_jwt(0);
    let env = E2ETestEnv::with_config(config)
        .await
        .expect("Failed to create test env");

    let users = ["user-alice", "user-bob", "user-charlie"];

    for user in users {
        let token = E2ETestEnv::generate_test_jwt(user, 3600);
        let response = env
            .put_object_with_jwt(
                &format!("/uploads/{}-file.txt", user),
                format!("content from {}", user).as_bytes(),
                &token,
            )
            .await
            .expect("Request failed");

        assert!(
            response.status().is_success(),
            "Upload from {} should succeed",
            user
        );
    }
}

/// Test: Token that expires during upload is handled gracefully
///
/// RED Phase: This test documents expected behavior when auth is enforced.
#[tokio::test]
#[ignore = "Server auth enforcement not yet implemented - RED phase test"]
async fn test_token_expiry_during_upload() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let config = E2ETestEnv::config_with_jwt(0);
    let env = E2ETestEnv::with_config(config)
        .await
        .expect("Failed to create test env");

    // Token that expires in 1 second
    let token = E2ETestEnv::generate_test_jwt("short-lived-user", 1);

    // First upload should succeed
    let response = env
        .put_object_with_jwt("/uploads/expiry-test-1.txt", b"quick upload", &token)
        .await
        .expect("Request failed");

    assert!(
        response.status().is_success(),
        "First upload should succeed before expiry"
    );

    // Wait for token to expire
    tokio::time::sleep(Duration::from_secs(2)).await;

    // Second upload should fail
    let response = env
        .put_object_with_jwt("/uploads/expiry-test-2.txt", b"late upload", &token)
        .await
        .expect("Request failed");

    assert_eq!(
        response.status().as_u16(),
        401,
        "Upload after token expiry should return 401"
    );
}
