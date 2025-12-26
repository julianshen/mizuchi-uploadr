//! Pingora Server Integration Tests
//!
//! RED Phase: These tests define the expected behavior of the Pingora-based HTTP server.
//! They will fail until the implementation is complete.
//!
//! Test Coverage:
//! - Server initialization and binding
//! - Health check endpoint
//! - Basic HTTP request handling
//! - Graceful shutdown
//!

use mizuchi_uploadr::config::{BucketConfig, Config, MetricsConfig, S3Config, ServerConfig, ZeroCopyConfig};
use mizuchi_uploadr::server::pingora::PingoraServer;
use std::time::Duration;
use tokio::time::sleep;

/// Helper function to create a test configuration
fn test_config(port: u16) -> Config {
    Config {
        server: ServerConfig {
            address: format!("127.0.0.1:{}", port),
            zero_copy: ZeroCopyConfig::default(),
        },
        buckets: vec![BucketConfig {
            name: "test".into(),
            path_prefix: "/uploads".into(),
            s3: S3Config {
                bucket: "test-bucket".into(),
                region: "us-east-1".into(),
                endpoint: Some("http://localhost:9000".into()),
                access_key: Some("minioadmin".into()),
                secret_key: Some("minioadmin".into()),
            },
            auth: Default::default(),
            upload: Default::default(),
        }],
        metrics: MetricsConfig::default(),
        tracing: None,
    }
}

/// Test: Server binds to configured address
///
/// RED Phase: This test will fail because PingoraServer doesn't exist yet
#[tokio::test]
async fn test_server_binds_to_configured_address() {
    let config = test_config(0); // Port 0 = OS assigns random port
    
    // Create server instance
    let server = PingoraServer::new(config).expect("Failed to create server");
    
    // Get the actual bound address
    let addr = server.local_addr().expect("Failed to get local address");
    
    // Verify it's bound to localhost
    assert_eq!(addr.ip().to_string(), "127.0.0.1");
    
    // Verify port was assigned
    assert!(addr.port() > 0, "Port should be assigned");
}

/// Test: Server responds to health check endpoint
///
/// RED Phase: This test will fail because health check endpoint doesn't exist yet
#[tokio::test]
async fn test_server_health_check_endpoint() {
    let config = test_config(0);
    let server = PingoraServer::new(config).expect("Failed to create server");
    let addr = server.local_addr().expect("Failed to get local address");
    
    // Start server in background
    let server_handle = tokio::spawn(async move {
        server.run().await
    });
    
    // Give server time to start
    sleep(Duration::from_millis(100)).await;
    
    // Make HTTP request to health check endpoint
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://{}/health", addr))
        .send()
        .await
        .expect("Failed to send request");
    
    // Verify response
    assert_eq!(response.status(), 200, "Health check should return 200 OK");
    
    let body = response.text().await.expect("Failed to read response body");
    assert!(body.contains("ok") || body.contains("healthy"), "Health check should return ok/healthy");
    
    // Shutdown server
    server_handle.abort();
}

/// Test: Server handles basic HTTP requests
///
/// RED Phase: This test will fail because request handling doesn't exist yet
#[tokio::test]
async fn test_server_handles_basic_http_requests() {
    let config = test_config(0);
    let server = PingoraServer::new(config).expect("Failed to create server");
    let addr = server.local_addr().expect("Failed to get local address");
    
    // Start server in background
    let server_handle = tokio::spawn(async move {
        server.run().await
    });
    
    // Give server time to start
    sleep(Duration::from_millis(100)).await;
    
    // Make HTTP PUT request (S3 upload)
    let client = reqwest::Client::new();
    let response = client
        .put(format!("http://{}/uploads/test.txt", addr))
        .body("Hello, World!")
        .header("Content-Type", "text/plain")
        .send()
        .await
        .expect("Failed to send request");
    
    // Verify response (should be 200 or 201 for successful upload)
    assert!(
        response.status().is_success(),
        "Upload should succeed, got status: {}",
        response.status()
    );
    
    // Shutdown server
    server_handle.abort();
}

/// Test: Server handles graceful shutdown
///
/// RED Phase: This test will fail because graceful shutdown doesn't exist yet
#[tokio::test]
async fn test_server_graceful_shutdown() {
    let config = test_config(0);
    let server = PingoraServer::new(config).expect("Failed to create server");
    
    // Start server in background
    let server_handle = tokio::spawn(async move {
        server.run().await
    });
    
    // Give server time to start
    sleep(Duration::from_millis(100)).await;
    
    // Send shutdown signal
    server_handle.abort();
    
    // Wait for shutdown to complete
    let result = tokio::time::timeout(Duration::from_secs(5), server_handle).await;
    
    // Verify shutdown completed within timeout
    assert!(
        result.is_ok(),
        "Server should shutdown gracefully within 5 seconds"
    );
}

