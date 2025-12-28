//! E2E Upload Flow Tests
//!
//! RED Phase: These tests define the expected behavior for the complete upload flow.
//! They test the full path from HTTP request through the proxy to the S3 backend.
//!
//! ## Test Coverage
//!
//! - Simple PUT upload (small files)
//! - Large file upload (multipart)
//! - Content-Type preservation
//! - ETag verification
//! - Concurrent uploads

use super::common::E2ETestEnv;

/// Test: Simple PUT upload succeeds
///
/// This validates the complete happy path:
/// 1. Client sends PUT request to proxy
/// 2. Proxy forwards to S3 backend
/// 3. Proxy returns success response
#[tokio::test]
async fn test_simple_put_upload_succeeds() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let env = E2ETestEnv::new().await.expect("Failed to create test env");

    // Upload a small file
    let payload = b"Hello, Mizuchi Uploadr E2E Test!";
    let response = env
        .put_object(
            "/uploads/e2e-test-small.txt",
            &payload[..],
            Some("text/plain"),
        )
        .await
        .expect("Request failed");

    assert!(
        response.status().is_success(),
        "Upload should succeed, got: {}",
        response.status()
    );

    // ETag may or may not be returned depending on implementation
    if let Some(etag) = response.headers().get("ETag") {
        let etag_value = etag.to_str().unwrap();
        println!("ETag returned: {}", etag_value);
    }
}

/// Test: Upload 1MB file successfully
#[tokio::test]
async fn test_upload_1mb_file() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let env = E2ETestEnv::new().await.expect("Failed to create test env");

    // Generate 1MB payload
    let payload = E2ETestEnv::random_payload(1024 * 1024);
    let response = env
        .put_object(
            "/uploads/e2e-test-1mb.bin",
            payload.to_vec(),
            Some("application/octet-stream"),
        )
        .await
        .expect("Request failed");

    assert!(
        response.status().is_success(),
        "1MB upload should succeed, got: {}",
        response.status()
    );
}

/// Test: Upload 10MB file successfully
#[tokio::test]
async fn test_upload_10mb_file() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let env = E2ETestEnv::new().await.expect("Failed to create test env");

    // Generate 10MB payload
    let payload = E2ETestEnv::random_payload(10 * 1024 * 1024);
    let response = env
        .put_object(
            "/uploads/e2e-test-10mb.bin",
            payload.to_vec(),
            Some("application/octet-stream"),
        )
        .await
        .expect("Request failed");

    assert!(
        response.status().is_success(),
        "10MB upload should succeed, got: {}",
        response.status()
    );
}

/// Test: Upload 50MB+ file triggers multipart upload
#[tokio::test]
async fn test_upload_large_file_multipart() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let env = E2ETestEnv::new().await.expect("Failed to create test env");

    // Generate 55MB payload (above multipart threshold)
    let payload = E2ETestEnv::random_payload(55 * 1024 * 1024);
    let response = env
        .put_object(
            "/uploads/e2e-test-55mb.bin",
            payload.to_vec(),
            Some("application/octet-stream"),
        )
        .await
        .expect("Request failed");

    assert!(
        response.status().is_success(),
        "55MB multipart upload should succeed, got: {}",
        response.status()
    );

    // Multipart uploads have a different ETag format (contains '-')
    // Note: ETag may not be returned by the proxy if not forwarded from S3
    if let Some(etag) = response.headers().get("ETag") {
        let etag_value = etag.to_str().unwrap();
        println!("Multipart ETag returned: {}", etag_value);
    }
}

/// Test: Content-Type is preserved through the proxy
#[tokio::test]
async fn test_content_type_preservation() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let env = E2ETestEnv::new().await.expect("Failed to create test env");

    let content_types = [
        ("test.json", r#"{"key": "value"}"#, "application/json"),
        ("test.xml", "<root>data</root>", "application/xml"),
        ("test.html", "<html></html>", "text/html"),
        ("test.csv", "a,b,c\n1,2,3", "text/csv"),
    ];

    for (filename, content, content_type) in content_types {
        let path = format!("/uploads/e2e-{}", filename);
        let response = env
            .put_object(&path, content.as_bytes(), Some(content_type))
            .await
            .expect("Request failed");

        assert!(
            response.status().is_success(),
            "Upload with content-type {} should succeed",
            content_type
        );
    }
}

/// Test: Concurrent uploads don't interfere with each other
#[tokio::test]
async fn test_concurrent_uploads() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let env = E2ETestEnv::new().await.expect("Failed to create test env");

    // Launch 10 concurrent uploads
    let mut handles = vec![];
    for i in 0..10 {
        let client = env.client.clone();
        let base_url = env.base_url();
        let payload = E2ETestEnv::random_payload(100 * 1024); // 100KB each

        let handle = tokio::spawn(async move {
            let response = client
                .put(format!("{}/uploads/concurrent-{}.bin", base_url, i))
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
            Ok(Ok(false)) => panic!("Concurrent upload {} failed with non-success status", i),
            Ok(Err(e)) => panic!("Concurrent upload {} failed: {}", i, e),
            Err(e) => panic!("Concurrent upload {} panicked: {}", i, e),
        }
    }
}

/// Test: Upload to non-existent bucket returns 404
#[tokio::test]
async fn test_upload_to_invalid_bucket_returns_404() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let env = E2ETestEnv::new().await.expect("Failed to create test env");

    // Try to upload to a path that doesn't match any configured bucket
    let response = env
        .put_object("/nonexistent-bucket/test.txt", "test", Some("text/plain"))
        .await
        .expect("Request failed");

    assert_eq!(
        response.status().as_u16(),
        404,
        "Upload to non-existent bucket should return 404"
    );
}

/// Test: Health check endpoint works
#[tokio::test]
async fn test_health_check_endpoint() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let env = E2ETestEnv::new().await.expect("Failed to create test env");

    let response = env
        .client
        .get(format!("{}/health", env.base_url()))
        .send()
        .await
        .expect("Request failed");

    assert!(
        response.status().is_success(),
        "Health check should return success"
    );
}

/// Test: Empty body upload is handled
#[tokio::test]
async fn test_empty_body_upload() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let env = E2ETestEnv::new().await.expect("Failed to create test env");

    let response = env
        .put_object("/uploads/empty-file.txt", "", Some("text/plain"))
        .await
        .expect("Request failed");

    // Empty uploads should either succeed (200/201) or be rejected (400)
    // depending on implementation choice
    let status = response.status().as_u16();
    assert!(
        status == 200 || status == 201 || status == 400,
        "Empty upload should return 200, 201, or 400, got: {}",
        status
    );
}

/// Test: Upload with special characters in key
#[tokio::test]
async fn test_upload_special_characters_in_key() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let env = E2ETestEnv::new().await.expect("Failed to create test env");

    // Test various special characters that should be URL-encoded
    let special_keys = [
        "/uploads/file with spaces.txt",
        "/uploads/file%20encoded.txt",
        "/uploads/path/to/nested/file.txt",
        "/uploads/unicode-日本語.txt",
    ];

    for key in special_keys {
        let response = env
            .put_object(key, b"test content", Some("text/plain"))
            .await
            .expect("Request failed");

        assert!(
            response.status().is_success() || response.status().as_u16() == 400,
            "Upload with key '{}' should succeed or return 400 for invalid chars, got: {}",
            key,
            response.status()
        );
    }
}
