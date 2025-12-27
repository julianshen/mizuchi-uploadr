//! OpenFGA Authorization Integration Tests
//!
//! Tests for OpenFGA fine-grained authorization using a mock server.

use mizuchi_uploadr::authz::openfga::{OpenFgaAuthorizer, OpenFgaConfig};
use mizuchi_uploadr::authz::{AuthzRequest, Authorizer};
use std::collections::HashMap;
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Helper to create an OpenFGA authorizer pointed at a mock server
fn create_authorizer(mock_server: &MockServer, store_id: &str) -> OpenFgaAuthorizer {
    let config = OpenFgaConfig {
        url: mock_server.uri(),
        store_id: store_id.to_string(),
        authorization_model_id: None,
        timeout: None,
        cache_ttl: None,
    };
    OpenFgaAuthorizer::new(config)
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
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/stores/test-store/check"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "allowed": true
            })))
            .mount(&mock_server)
            .await;

        let authorizer = create_authorizer(&mock_server, "test-store");
        let request = create_request("alice", "upload", "uploads/file.txt");

        let result = authorizer.authorize(&request).await;

        assert!(result.is_ok());
        assert!(result.unwrap(), "Expected allow decision");
    }

    #[tokio::test]
    async fn test_deny_decision_returned() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/stores/test-store/check"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "allowed": false
            })))
            .mount(&mock_server)
            .await;

        let authorizer = create_authorizer(&mock_server, "test-store");
        let request = create_request("bob", "upload", "private/secret.txt");

        let result = authorizer.authorize(&request).await;

        assert!(result.is_ok());
        assert!(!result.unwrap(), "Expected deny decision");
    }

    #[tokio::test]
    async fn test_server_error_returns_backend_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/stores/test-store/check"))
            .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
            .mount(&mock_server)
            .await;

        let authorizer = create_authorizer(&mock_server, "test-store");
        let request = create_request("alice", "upload", "uploads/file.txt");

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
    async fn test_connection_refused() {
        let config = OpenFgaConfig {
            url: "http://127.0.0.1:19998".to_string(),
            store_id: "test-store".to_string(),
            authorization_model_id: None,
            timeout: Some(std::time::Duration::from_millis(100)),
            cache_ttl: None,
        };
        let authorizer = OpenFgaAuthorizer::new(config);
        let request = create_request("alice", "upload", "uploads/file.txt");

        let result = authorizer.authorize(&request).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_invalid_json_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/stores/test-store/check"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not valid json"))
            .mount(&mock_server)
            .await;

        let authorizer = create_authorizer(&mock_server, "test-store");
        let request = create_request("alice", "upload", "uploads/file.txt");

        let result = authorizer.authorize(&request).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_request_body_structure() {
        let mock_server = MockServer::start().await;

        // Verify the exact request body structure for OpenFGA
        Mock::given(method("POST"))
            .and(path("/stores/test-store/check"))
            .and(body_json(json!({
                "tuple_key": {
                    "user": "user:alice",
                    "relation": "writer",
                    "object": "bucket:uploads/file.txt"
                }
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "allowed": true
            })))
            .mount(&mock_server)
            .await;

        let authorizer = create_authorizer(&mock_server, "test-store");
        let request = create_request("alice", "upload", "uploads/file.txt");

        let result = authorizer.authorize(&request).await;

        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_action_to_relation_mapping() {
        let mock_server = MockServer::start().await;

        // Test write action maps to writer relation
        Mock::given(method("POST"))
            .and(path("/stores/test-store/check"))
            .and(body_json(json!({
                "tuple_key": {
                    "user": "user:alice",
                    "relation": "writer",
                    "object": "bucket:uploads/file.txt"
                }
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "allowed": true })))
            .mount(&mock_server)
            .await;

        let authorizer = create_authorizer(&mock_server, "test-store");

        // "write" should map to "writer" relation
        let request = create_request("alice", "write", "uploads/file.txt");
        let result = authorizer.authorize(&request).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_authorization_model_id_included() {
        let mock_server = MockServer::start().await;

        // When authorization_model_id is set, it should be included in request
        Mock::given(method("POST"))
            .and(path("/stores/test-store/check"))
            .and(body_json(json!({
                "tuple_key": {
                    "user": "user:alice",
                    "relation": "writer",
                    "object": "bucket:uploads/file.txt"
                },
                "authorization_model_id": "model-123"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "allowed": true })))
            .mount(&mock_server)
            .await;

        let config = OpenFgaConfig {
            url: mock_server.uri(),
            store_id: "test-store".to_string(),
            authorization_model_id: Some("model-123".to_string()),
            timeout: None,
            cache_ttl: None,
        };
        let authorizer = OpenFgaAuthorizer::new(config);
        let request = create_request("alice", "upload", "uploads/file.txt");

        let result = authorizer.authorize(&request).await;

        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    // === NEW FEATURES TO BE IMPLEMENTED (RED phase) ===

    #[tokio::test]
    async fn test_timeout_configuration() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/stores/test-store/check"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({ "allowed": true }))
                    .set_delay(std::time::Duration::from_secs(5)),
            )
            .mount(&mock_server)
            .await;

        // Create authorizer with 100ms timeout
        let config = OpenFgaConfig {
            url: mock_server.uri(),
            store_id: "test-store".to_string(),
            authorization_model_id: None,
            timeout: Some(std::time::Duration::from_millis(100)),
            cache_ttl: None,
        };
        let authorizer = OpenFgaAuthorizer::new(config);
        let request = create_request("alice", "upload", "uploads/file.txt");

        let result = authorizer.authorize(&request).await;

        assert!(result.is_err(), "Should timeout");
    }

    #[tokio::test]
    async fn test_response_caching_avoids_repeated_calls() {
        use std::sync::atomic::{AtomicU32, Ordering};
        use std::sync::Arc;

        let mock_server = MockServer::start().await;
        let call_count = Arc::new(AtomicU32::new(0));
        let call_count_clone = call_count.clone();

        Mock::given(method("POST"))
            .and(path("/stores/test-store/check"))
            .respond_with(move |_: &wiremock::Request| {
                call_count_clone.fetch_add(1, Ordering::SeqCst);
                ResponseTemplate::new(200).set_body_json(json!({ "allowed": true }))
            })
            .mount(&mock_server)
            .await;

        let config = OpenFgaConfig {
            url: mock_server.uri(),
            store_id: "test-store".to_string(),
            authorization_model_id: None,
            timeout: None,
            cache_ttl: Some(std::time::Duration::from_secs(1)),
        };
        let authorizer = OpenFgaAuthorizer::new(config);
        let request = create_request("alice", "upload", "uploads/file.txt");

        for _ in 0..5 {
            let result = authorizer.authorize(&request).await;
            assert!(result.is_ok());
            assert!(result.unwrap());
        }

        assert_eq!(
            call_count.load(Ordering::SeqCst),
            1,
            "Should only call OpenFGA once due to caching"
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
            .and(path("/stores/test-store/check"))
            .respond_with(move |_: &wiremock::Request| {
                call_count_clone.fetch_add(1, Ordering::SeqCst);
                ResponseTemplate::new(200).set_body_json(json!({ "allowed": true }))
            })
            .mount(&mock_server)
            .await;

        let config = OpenFgaConfig {
            url: mock_server.uri(),
            store_id: "test-store".to_string(),
            authorization_model_id: None,
            timeout: None,
            cache_ttl: Some(std::time::Duration::from_millis(50)),
        };
        let authorizer = OpenFgaAuthorizer::new(config);
        let request = create_request("alice", "upload", "uploads/file.txt");

        // First call
        authorizer.authorize(&request).await.unwrap();
        assert_eq!(call_count.load(Ordering::SeqCst), 1);

        // Wait for cache to expire
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Second call should hit OpenFGA again
        authorizer.authorize(&request).await.unwrap();
        assert_eq!(
            call_count.load(Ordering::SeqCst),
            2,
            "Should call OpenFGA again after TTL expires"
        );
    }

    #[tokio::test]
    async fn test_builder_pattern_configuration() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/stores/test-store/check"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "allowed": true })))
            .mount(&mock_server)
            .await;

        let authorizer = OpenFgaAuthorizer::builder()
            .url(&mock_server.uri())
            .store_id("test-store")
            .timeout(std::time::Duration::from_secs(5))
            .cache_ttl(std::time::Duration::from_secs(60))
            .build()
            .expect("Should build authorizer");

        let request = create_request("alice", "upload", "uploads/file.txt");
        let result = authorizer.authorize(&request).await;

        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[tokio::test]
    async fn test_batch_check() {
        let mock_server = MockServer::start().await;

        // OpenFGA batch check endpoint
        Mock::given(method("POST"))
            .and(path("/stores/test-store/batch-check"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "results": [
                    { "allowed": true },
                    { "allowed": false },
                    { "allowed": true }
                ]
            })))
            .mount(&mock_server)
            .await;

        let authorizer = OpenFgaAuthorizer::builder()
            .url(&mock_server.uri())
            .store_id("test-store")
            .build()
            .expect("Should build authorizer");

        let requests = vec![
            create_request("alice", "upload", "file1.txt"),
            create_request("bob", "upload", "file2.txt"),
            create_request("charlie", "upload", "file3.txt"),
        ];

        let results = authorizer.batch_check(&requests).await;

        assert!(results.is_ok());
        let results = results.unwrap();
        assert_eq!(results.len(), 3);
        assert!(results[0]);
        assert!(!results[1]);
        assert!(results[2]);
    }
}
