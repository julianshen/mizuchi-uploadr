//! OPA Authorization Integration Tests
//!
//! Tests for OPA policy evaluation using a mock server.

use mizuchi_uploadr::authz::opa::{OpaAuthorizer, OpaConfig};
use mizuchi_uploadr::authz::{AuthzRequest, Authorizer};
use std::collections::HashMap;
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper to create an OPA authorizer pointed at a mock server
fn create_authorizer(mock_server: &MockServer, policy_path: &str) -> OpaAuthorizer {
    let config = OpaConfig {
        url: mock_server.uri(),
        policy_path: policy_path.to_string(),
        timeout: None,
        cache_ttl: None, // No caching for basic tests
    };
    OpaAuthorizer::new(config)
}

/// Helper to create a test authorization request
fn create_request(subject: &str, action: &str, resource: &str) -> AuthzRequest {
    AuthzRequest {
        subject: subject.to_string(),
        action: action.to_string(),
        resource: resource.to_string(),
        context: HashMap::new(),
    }
}

mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_allow_decision_returned() {
        // Setup mock server to return allow
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/data/mizuchi/allow"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "result": true
            })))
            .mount(&mock_server)
            .await;

        let authorizer = create_authorizer(&mock_server, "mizuchi/allow");
        let request = create_request("user:alice", "upload", "bucket/uploads/file.txt");

        let result = authorizer.authorize(&request).await;

        assert!(result.is_ok());
        assert!(result.unwrap(), "Expected allow decision");
    }

    #[tokio::test]
    async fn test_deny_decision_returned() {
        // Setup mock server to return deny
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/data/mizuchi/allow"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "result": false
            })))
            .mount(&mock_server)
            .await;

        let authorizer = create_authorizer(&mock_server, "mizuchi/allow");
        let request = create_request("user:bob", "upload", "bucket/private/secret.txt");

        let result = authorizer.authorize(&request).await;

        assert!(result.is_ok());
        assert!(!result.unwrap(), "Expected deny decision");
    }

    #[tokio::test]
    async fn test_missing_result_defaults_to_deny() {
        // OPA may return empty result when policy doesn't match
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/data/mizuchi/allow"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .mount(&mock_server)
            .await;

        let authorizer = create_authorizer(&mock_server, "mizuchi/allow");
        let request = create_request("user:unknown", "delete", "bucket/uploads/file.txt");

        let result = authorizer.authorize(&request).await;

        assert!(result.is_ok());
        assert!(!result.unwrap(), "Missing result should default to deny");
    }

    #[tokio::test]
    async fn test_null_result_defaults_to_deny() {
        // OPA returns null when policy doesn't evaluate to a value
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/data/mizuchi/allow"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "result": null
            })))
            .mount(&mock_server)
            .await;

        let authorizer = create_authorizer(&mock_server, "mizuchi/allow");
        let request = create_request("user:unknown", "read", "bucket/data/file.txt");

        let result = authorizer.authorize(&request).await;

        assert!(result.is_ok());
        assert!(!result.unwrap(), "Null result should default to deny");
    }

    #[tokio::test]
    async fn test_opa_server_error_returns_backend_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/data/mizuchi/allow"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&mock_server)
            .await;

        let authorizer = create_authorizer(&mock_server, "mizuchi/allow");
        let request = create_request("user:alice", "upload", "bucket/uploads/file.txt");

        let result = authorizer.authorize(&request).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("500"),
            "Error should mention status code: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_opa_not_found_returns_backend_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/data/mizuchi/nonexistent"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
            .mount(&mock_server)
            .await;

        let authorizer = create_authorizer(&mock_server, "mizuchi/nonexistent");
        let request = create_request("user:alice", "upload", "bucket/uploads/file.txt");

        let result = authorizer.authorize(&request).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("404"),
            "Error should mention status code: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_opa_connection_refused() {
        // Use a port that's definitely not listening
        let config = OpaConfig {
            url: "http://127.0.0.1:19999".to_string(),
            policy_path: "mizuchi/allow".to_string(),
            timeout: Some(std::time::Duration::from_millis(100)), // Short timeout
            cache_ttl: None,
        };
        let authorizer = OpaAuthorizer::new(config);
        let request = create_request("user:alice", "upload", "bucket/uploads/file.txt");

        let result = authorizer.authorize(&request).await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("error") || err.to_string().contains("connect"),
            "Error should indicate connection failure: {}",
            err
        );
    }

    #[tokio::test]
    async fn test_invalid_json_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/data/mizuchi/allow"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not valid json"))
            .mount(&mock_server)
            .await;

        let authorizer = create_authorizer(&mock_server, "mizuchi/allow");
        let request = create_request("user:alice", "upload", "bucket/uploads/file.txt");

        let result = authorizer.authorize(&request).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_request_contains_correct_input_structure() {
        let mock_server = MockServer::start().await;

        // Verify the exact request body structure
        Mock::given(method("POST"))
            .and(path("/v1/data/mizuchi/allow"))
            .and(body_json(json!({
                "input": {
                    "subject": "user:alice",
                    "action": "upload",
                    "resource": "bucket/uploads/file.txt"
                }
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "result": true
            })))
            .mount(&mock_server)
            .await;

        let authorizer = create_authorizer(&mock_server, "mizuchi/allow");
        let request = create_request("user:alice", "upload", "bucket/uploads/file.txt");

        let result = authorizer.authorize(&request).await;

        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_context_included_in_request() {
        let mock_server = MockServer::start().await;

        // Verify context is passed through
        Mock::given(method("POST"))
            .and(path("/v1/data/mizuchi/allow"))
            .and(body_json(json!({
                "input": {
                    "subject": "user:alice",
                    "action": "upload",
                    "resource": "bucket/uploads/file.txt",
                    "ip_address": "192.168.1.1",
                    "department": "engineering"
                }
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "result": true
            })))
            .mount(&mock_server)
            .await;

        let authorizer = create_authorizer(&mock_server, "mizuchi/allow");
        let mut request = create_request("user:alice", "upload", "bucket/uploads/file.txt");
        request
            .context
            .insert("ip_address".to_string(), json!("192.168.1.1"));
        request
            .context
            .insert("department".to_string(), json!("engineering"));

        let result = authorizer.authorize(&request).await;

        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_different_policy_paths() {
        let mock_server = MockServer::start().await;

        // Test custom policy path
        Mock::given(method("POST"))
            .and(path("/v1/data/custom/policy/v2/upload"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "result": true
            })))
            .mount(&mock_server)
            .await;

        let authorizer = create_authorizer(&mock_server, "custom/policy/v2/upload");
        let request = create_request("user:alice", "upload", "bucket/uploads/file.txt");

        let result = authorizer.authorize(&request).await;

        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    // === NEW FEATURES TO BE IMPLEMENTED (RED phase) ===

    #[tokio::test]
    async fn test_timeout_configuration() {
        // Slow server should trigger timeout
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/data/mizuchi/allow"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "result": true }))
                    .set_delay(std::time::Duration::from_secs(5)), // 5 second delay
            )
            .mount(&mock_server)
            .await;

        // Create authorizer with 100ms timeout
        let config = OpaConfig {
            url: mock_server.uri(),
            policy_path: "mizuchi/allow".to_string(),
            timeout: Some(std::time::Duration::from_millis(100)),
            cache_ttl: None,
        };
        let authorizer = OpaAuthorizer::new(config);
        let request = create_request("user:alice", "upload", "bucket/uploads/file.txt");

        let result = authorizer.authorize(&request).await;

        assert!(result.is_err(), "Should timeout");
        // The request should fail due to timeout (fast failure instead of 5 second wait)
        // Error message may vary but important thing is the request didn't succeed
    }

    #[tokio::test]
    async fn test_response_caching_avoids_repeated_calls() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let mock_server = MockServer::start().await;
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        // Track how many times the mock is called
        Mock::given(method("POST"))
            .and(path("/v1/data/mizuchi/allow"))
            .respond_with(move |_: &wiremock::Request| {
                call_count_clone.fetch_add(1, Ordering::SeqCst);
                ResponseTemplate::new(200).set_body_json(json!({ "result": true }))
            })
            .mount(&mock_server)
            .await;

        // Create authorizer with caching enabled (1 second TTL)
        let config = OpaConfig {
            url: mock_server.uri(),
            policy_path: "mizuchi/allow".to_string(),
            timeout: None,
            cache_ttl: Some(std::time::Duration::from_secs(1)),
        };
        let authorizer = OpaAuthorizer::new(config);

        // Same request multiple times
        let request = create_request("user:alice", "upload", "bucket/uploads/file.txt");

        for _ in 0..5 {
            let result = authorizer.authorize(&request).await;
            assert!(result.is_ok());
            assert!(result.unwrap());
        }

        // Should only have called OPA once due to caching
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            1,
            "Should only call OPA once due to caching"
        );
    }

    #[tokio::test]
    async fn test_cache_respects_ttl() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let mock_server = MockServer::start().await;
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        Mock::given(method("POST"))
            .and(path("/v1/data/mizuchi/allow"))
            .respond_with(move |_: &wiremock::Request| {
                call_count_clone.fetch_add(1, Ordering::SeqCst);
                ResponseTemplate::new(200).set_body_json(json!({ "result": true }))
            })
            .mount(&mock_server)
            .await;

        // Short TTL for testing
        let config = OpaConfig {
            url: mock_server.uri(),
            policy_path: "mizuchi/allow".to_string(),
            timeout: None,
            cache_ttl: Some(std::time::Duration::from_millis(50)),
        };
        let authorizer = OpaAuthorizer::new(config);
        let request = create_request("user:alice", "upload", "bucket/uploads/file.txt");

        // First call
        authorizer.authorize(&request).await.unwrap();
        assert_eq!(call_count.load(Ordering::SeqCst), 1);

        // Wait for cache to expire
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Second call should hit OPA again
        authorizer.authorize(&request).await.unwrap();
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            2,
            "Should call OPA again after TTL expires"
        );
    }

    #[tokio::test]
    async fn test_cache_different_requests_not_shared() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let mock_server = MockServer::start().await;
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        Mock::given(method("POST"))
            .and(path("/v1/data/mizuchi/allow"))
            .respond_with(move |_: &wiremock::Request| {
                call_count_clone.fetch_add(1, Ordering::SeqCst);
                ResponseTemplate::new(200).set_body_json(json!({ "result": true }))
            })
            .mount(&mock_server)
            .await;

        let config = OpaConfig {
            url: mock_server.uri(),
            policy_path: "mizuchi/allow".to_string(),
            timeout: None,
            cache_ttl: Some(std::time::Duration::from_secs(60)),
        };
        let authorizer = OpaAuthorizer::new(config);

        // Different requests
        let request1 = create_request("user:alice", "upload", "bucket/uploads/file1.txt");
        let request2 = create_request("user:alice", "upload", "bucket/uploads/file2.txt");
        let request3 = create_request("user:bob", "upload", "bucket/uploads/file1.txt");

        authorizer.authorize(&request1).await.unwrap();
        authorizer.authorize(&request2).await.unwrap();
        authorizer.authorize(&request3).await.unwrap();

        // Each unique request should call OPA
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            3,
            "Different requests should not share cache"
        );
    }

    #[tokio::test]
    async fn test_builder_pattern_configuration() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/v1/data/mizuchi/allow"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "result": true })))
            .mount(&mock_server)
            .await;

        // Use builder pattern for configuration
        let authorizer = OpaAuthorizer::builder()
            .url(&mock_server.uri())
            .policy_path("mizuchi/allow")
            .timeout(std::time::Duration::from_secs(5))
            .cache_ttl(std::time::Duration::from_secs(60))
            .build()
            .expect("Should build authorizer");

        let request = create_request("user:alice", "upload", "bucket/uploads/file.txt");
        let result = authorizer.authorize(&request).await;

        assert!(result.is_ok());
        assert!(result.unwrap());
    }
}
