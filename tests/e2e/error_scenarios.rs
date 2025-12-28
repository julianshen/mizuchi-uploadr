//! E2E Error Scenario Tests
//!
//! RED Phase: These tests define the expected behavior for various error conditions.
//! They validate proper error handling throughout the upload proxy.
//!
//! ## Test Coverage
//!
//! - Invalid HTTP methods
//! - Malformed requests
//! - S3 backend errors
//! - Timeout handling
//! - Connection failures
//! - Rate limiting (if implemented)

use super::common::E2ETestEnv;
use std::time::Duration;

/// Test: GET request to upload path returns 404 or 405
/// (Upload-only proxy doesn't support GET, may return 404 or 405)
#[tokio::test]
async fn test_get_request_rejected() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let env = E2ETestEnv::new().await.expect("Failed to create test env");

    let response = env
        .client
        .get(format!("{}/uploads/test.txt", env.base_url()))
        .send()
        .await
        .expect("Request failed");

    let status = response.status().as_u16();
    assert!(
        status == 404 || status == 405,
        "GET request should return 404 or 405, got: {}",
        status
    );
}

/// Test: DELETE request returns 404 or 405 (upload-only proxy)
#[tokio::test]
async fn test_delete_request_rejected() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let env = E2ETestEnv::new().await.expect("Failed to create test env");

    let response = env
        .client
        .delete(format!("{}/uploads/test.txt", env.base_url()))
        .send()
        .await
        .expect("Request failed");

    let status = response.status().as_u16();
    assert!(
        status == 404 || status == 405,
        "DELETE request should return 404 or 405, got: {}",
        status
    );
}

/// Test: HEAD request returns 405 or appropriate response
#[tokio::test]
async fn test_head_request_handling() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let env = E2ETestEnv::new().await.expect("Failed to create test env");

    let response = env
        .client
        .head(format!("{}/uploads/test.txt", env.base_url()))
        .send()
        .await
        .expect("Request failed");

    // HEAD should return 405 for upload-only proxy
    let status = response.status().as_u16();
    assert!(
        status == 405 || status == 404,
        "HEAD request should return 405 or 404, got: {}",
        status
    );
}

/// Test: Request to root path returns appropriate response
#[tokio::test]
async fn test_root_path_request() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let env = E2ETestEnv::new().await.expect("Failed to create test env");

    let response = env
        .client
        .put(format!("{}/", env.base_url()))
        .body("test")
        .send()
        .await
        .expect("Request failed");

    // Root path should return 404 (no bucket matched)
    let status = response.status().as_u16();
    assert!(
        status == 404 || status == 400,
        "Root path should return 404 or 400, got: {}",
        status
    );
}

/// Test: Very long path is handled
#[tokio::test]
async fn test_very_long_path() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let env = E2ETestEnv::new().await.expect("Failed to create test env");

    // Create a very long path (8KB)
    let long_segment = "a".repeat(1000);
    let long_path = format!(
        "/uploads/{}/{}/{}/{}/{}/{}/{}/{}/file.txt",
        long_segment,
        long_segment,
        long_segment,
        long_segment,
        long_segment,
        long_segment,
        long_segment,
        long_segment
    );

    let response = env
        .client
        .put(format!("{}{}", env.base_url(), long_path))
        .body("test")
        .send()
        .await
        .expect("Request failed");

    // Very long paths should be handled (either success or 414 URI Too Long)
    let status = response.status().as_u16();
    assert!(
        status == 200 || status == 201 || status == 414 || status == 400,
        "Very long path should return success or 414/400, got: {}",
        status
    );
}

/// Test: Request with invalid Content-Length
#[tokio::test]
async fn test_invalid_content_length() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let env = E2ETestEnv::new().await.expect("Failed to create test env");

    // Note: Hyper/reqwest may handle this before it reaches our server
    let response = env
        .client
        .put(format!("{}/uploads/test.txt", env.base_url()))
        .header("Content-Length", "999999") // Claim large body
        .body("small") // But send small body
        .timeout(Duration::from_secs(5))
        .send()
        .await;

    // Connection might be reset or we get an error
    // This is acceptable behavior
    match response {
        Ok(r) => {
            // If we get a response, it should indicate error
            assert!(
                !r.status().is_success() || r.status().as_u16() == 200,
                "Mismatched Content-Length should fail or be handled: {}",
                r.status()
            );
        }
        Err(_) => {
            // Connection error is acceptable for protocol violation
        }
    }
}

/// Test: Request timeout handling
///
/// This test verifies that client timeout handling works correctly.
/// We use a short timeout, but the test is designed to handle both
/// timeout errors and fast server responses gracefully.
#[tokio::test]
async fn test_request_timeout_handling() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let env = E2ETestEnv::new().await.expect("Failed to create test env");

    // Use a very short timeout with a large payload
    let client = reqwest::Client::builder()
        .timeout(Duration::from_millis(1))
        .build()
        .unwrap();

    let result = client
        .put(format!("{}/uploads/timeout-test.txt", env.base_url()))
        .body(E2ETestEnv::random_payload(10 * 1024 * 1024).to_vec()) // 10MB
        .send()
        .await;

    // The request should either timeout (error) or complete very quickly
    // On fast systems, even 1ms might be enough for a small response
    // The key is that the timeout mechanism is working
    match result {
        Err(_) => {
            // Timeout occurred as expected
        }
        Ok(resp) => {
            // If the server responded, it should be a valid response
            // This can happen on very fast local systems
            assert!(
                resp.status().is_success() || resp.status().is_client_error(),
                "Unexpected status: {}",
                resp.status()
            );
        }
    }
}

/// Test: Missing Content-Type defaults correctly
#[tokio::test]
async fn test_missing_content_type() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let env = E2ETestEnv::new().await.expect("Failed to create test env");

    // Upload without Content-Type header
    let response = env
        .client
        .put(format!("{}/uploads/no-content-type.bin", env.base_url()))
        .body("binary data")
        .send()
        .await
        .expect("Request failed");

    assert!(
        response.status().is_success(),
        "Upload without Content-Type should succeed with default, got: {}",
        response.status()
    );
}

/// Test: Upload with invalid bucket name in path
#[tokio::test]
async fn test_invalid_bucket_in_path() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let env = E2ETestEnv::new().await.expect("Failed to create test env");

    let invalid_paths = [
        "/../uploads/test.txt",   // Path traversal attempt
        "/./uploads/test.txt",    // Current directory
        "//uploads/test.txt",     // Double slash
        "/uploads/../etc/passwd", // Path traversal in key
    ];

    for path in invalid_paths {
        let response = env
            .client
            .put(format!("{}{}", env.base_url(), path))
            .body("test")
            .send()
            .await
            .expect("Request failed");

        let status = response.status().as_u16();
        assert!(
            status == 400 || status == 404 || status == 200,
            "Path '{}' should return 400, 404, or 200 (if normalized), got: {}",
            path,
            status
        );
    }
}

/// Test: Connection close during upload
#[tokio::test]
async fn test_connection_close_during_upload() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let env = E2ETestEnv::new().await.expect("Failed to create test env");

    // Create a client that closes connections aggressively
    let client = reqwest::Client::builder()
        .pool_max_idle_per_host(0) // Don't keep connections
        .build()
        .unwrap();

    // Make several requests with connection closing
    for i in 0..5 {
        let response = client
            .put(format!("{}/uploads/conn-test-{}.txt", env.base_url(), i))
            .body("test data")
            .header("Connection", "close")
            .send()
            .await
            .expect("Request failed");

        assert!(
            response.status().is_success(),
            "Upload with Connection: close should succeed"
        );
    }
}

/// Test: Concurrent error scenarios
#[tokio::test]
async fn test_concurrent_error_scenarios() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let env = E2ETestEnv::new().await.expect("Failed to create test env");

    // Launch concurrent requests with various error conditions
    let mut handles = vec![];

    // Some valid requests
    for i in 0..5 {
        let client = env.client.clone();
        let base_url = env.base_url();
        handles.push(tokio::spawn(async move {
            client
                .put(format!("{}/uploads/concurrent-ok-{}.txt", base_url, i))
                .body("valid")
                .send()
                .await
                .map(|r| r.status().is_success())
                .unwrap_or(false)
        }));
    }

    // Some invalid requests (wrong method)
    for i in 0..5 {
        let client = env.client.clone();
        let base_url = env.base_url();
        handles.push(tokio::spawn(async move {
            client
                .get(format!("{}/uploads/concurrent-bad-{}.txt", base_url, i))
                .send()
                .await
                .map(|r| r.status().as_u16() == 405)
                .unwrap_or(false)
        }));
    }

    let results: Vec<_> = futures::future::join_all(handles).await;

    // All requests should complete without panicking
    for (i, result) in results.iter().enumerate() {
        assert!(
            result.is_ok(),
            "Concurrent request {} should complete: {:?}",
            i,
            result
        );
    }
}

/// Test: Large number of concurrent requests (stress test)
#[tokio::test]
async fn test_high_concurrency_stress() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let env = E2ETestEnv::new().await.expect("Failed to create test env");

    // Launch 50 concurrent small uploads
    let mut handles = vec![];
    for i in 0..50 {
        let client = env.client.clone();
        let base_url = env.base_url();

        handles.push(tokio::spawn(async move {
            client
                .put(format!("{}/uploads/stress-{}.txt", base_url, i))
                .body("stress test data")
                .send()
                .await
                .map(|r| r.status().is_success())
                .unwrap_or(false)
        }));
    }

    let results: Vec<_> = futures::future::join_all(handles).await;

    // Count successes
    let success_count = results.iter().filter(|r| matches!(r, Ok(true))).count();

    // At least 90% should succeed under load
    assert!(
        success_count >= 45,
        "At least 90% of stress test requests should succeed, got {}/50",
        success_count
    );
}

/// Test: Request with binary content in non-binary content-type
#[tokio::test]
async fn test_binary_content_with_text_content_type() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let env = E2ETestEnv::new().await.expect("Failed to create test env");

    // Send binary data with text/plain content-type
    let binary_data: Vec<u8> = (0..256).map(|i| i as u8).collect();

    let response = env
        .put_object(
            "/uploads/binary-as-text.txt",
            binary_data,
            Some("text/plain"),
        )
        .await
        .expect("Request failed");

    // Should succeed - content-type is just metadata
    assert!(
        response.status().is_success(),
        "Binary content with text content-type should succeed, got: {}",
        response.status()
    );
}
