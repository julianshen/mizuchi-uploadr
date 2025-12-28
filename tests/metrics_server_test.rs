//! Metrics Server Integration Tests
//!
//! Tests for Prometheus metrics HTTP endpoint.

use std::time::Duration;

mod tests {
    use super::*;

    #[tokio::test]
    async fn test_metrics_server_starts_on_configured_port() {
        use mizuchi_uploadr::metrics::server::{MetricsServer, MetricsServerConfig};

        let config = MetricsServerConfig {
            address: "127.0.0.1:0".to_string(), // Use port 0 for random available port
        };

        let mut server = MetricsServer::new(config);
        let addr = server.start().await.expect("Server should start");

        // Server should be listening
        let client = reqwest::Client::new();
        let response = client
            .get(format!("http://{}/metrics", addr))
            .timeout(Duration::from_secs(5))
            .send()
            .await
            .expect("Should connect to metrics server");

        assert!(response.status().is_success());

        server.shutdown().await;
    }

    #[tokio::test]
    async fn test_metrics_endpoint_returns_prometheus_format() {
        use mizuchi_uploadr::metrics::server::{MetricsServer, MetricsServerConfig};

        let config = MetricsServerConfig {
            address: "127.0.0.1:0".to_string(),
        };

        let mut server = MetricsServer::new(config);
        let addr = server.start().await.expect("Server should start");

        let client = reqwest::Client::new();
        let response = client
            .get(format!("http://{}/metrics", addr))
            .send()
            .await
            .expect("Should get metrics");

        let content_type = response
            .headers()
            .get("content-type")
            .expect("Should have content-type");

        // Prometheus text format
        assert!(
            content_type.to_str().unwrap().contains("text/plain")
                || content_type
                    .to_str()
                    .unwrap()
                    .contains("text/plain; version=0.0.4"),
            "Content-Type should be Prometheus text format"
        );

        let body = response.text().await.unwrap();

        // Should contain our custom metrics
        assert!(
            body.contains("mizuchi_uploads_total") || body.contains("# HELP"),
            "Should contain Prometheus metrics format"
        );

        server.shutdown().await;
    }

    #[tokio::test]
    async fn test_metrics_include_upload_counters() {
        use mizuchi_uploadr::metrics;
        use mizuchi_uploadr::metrics::server::{MetricsServer, MetricsServerConfig};

        let config = MetricsServerConfig {
            address: "127.0.0.1:0".to_string(),
        };

        let mut server = MetricsServer::new(config);
        let addr = server.start().await.expect("Server should start");

        // Record some metrics
        metrics::record_upload_success("test-bucket", 1024);
        metrics::record_upload_success("test-bucket", 2048);
        metrics::record_upload_failure("test-bucket");

        let client = reqwest::Client::new();
        let body = client
            .get(format!("http://{}/metrics", addr))
            .send()
            .await
            .expect("Should get metrics")
            .text()
            .await
            .unwrap();

        // Verify upload metrics are present
        assert!(
            body.contains("mizuchi_uploads_total"),
            "Should contain upload counter"
        );
        assert!(
            body.contains("mizuchi_upload_bytes_total"),
            "Should contain bytes counter"
        );

        server.shutdown().await;
    }

    #[tokio::test]
    async fn test_metrics_include_auth_counters() {
        use mizuchi_uploadr::metrics;
        use mizuchi_uploadr::metrics::server::{MetricsServer, MetricsServerConfig};

        let config = MetricsServerConfig {
            address: "127.0.0.1:0".to_string(),
        };

        let mut server = MetricsServer::new(config);
        let addr = server.start().await.expect("Server should start");

        // Record auth metrics
        metrics::record_auth_attempt("jwt", true);
        metrics::record_auth_attempt("sigv4", false);

        let client = reqwest::Client::new();
        let body = client
            .get(format!("http://{}/metrics", addr))
            .send()
            .await
            .expect("Should get metrics")
            .text()
            .await
            .unwrap();

        assert!(
            body.contains("mizuchi_auth_attempts_total"),
            "Should contain auth counter"
        );

        server.shutdown().await;
    }

    #[tokio::test]
    async fn test_metrics_include_error_counters() {
        use mizuchi_uploadr::metrics;
        use mizuchi_uploadr::metrics::server::{MetricsServer, MetricsServerConfig};

        let config = MetricsServerConfig {
            address: "127.0.0.1:0".to_string(),
        };

        let mut server = MetricsServer::new(config);
        let addr = server.start().await.expect("Server should start");

        // Record error metrics
        metrics::record_error("s3_error");
        metrics::record_error("auth_error");

        let client = reqwest::Client::new();
        let body = client
            .get(format!("http://{}/metrics", addr))
            .send()
            .await
            .expect("Should get metrics")
            .text()
            .await
            .unwrap();

        assert!(
            body.contains("mizuchi_errors_total"),
            "Should contain error counter"
        );

        server.shutdown().await;
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        use mizuchi_uploadr::metrics::server::{MetricsServer, MetricsServerConfig};

        let config = MetricsServerConfig {
            address: "127.0.0.1:0".to_string(),
        };

        let mut server = MetricsServer::new(config);
        let addr = server.start().await.expect("Server should start");

        let client = reqwest::Client::new();
        let response = client
            .get(format!("http://{}/health", addr))
            .send()
            .await
            .expect("Should get health");

        assert!(response.status().is_success());

        let body = response.text().await.unwrap();
        assert!(body.contains("ok") || body.contains("healthy"));

        server.shutdown().await;
    }

    #[tokio::test]
    async fn test_unknown_endpoint_returns_404() {
        use mizuchi_uploadr::metrics::server::{MetricsServer, MetricsServerConfig};

        let config = MetricsServerConfig {
            address: "127.0.0.1:0".to_string(),
        };

        let mut server = MetricsServer::new(config);
        let addr = server.start().await.expect("Server should start");

        let client = reqwest::Client::new();
        let response = client
            .get(format!("http://{}/unknown", addr))
            .send()
            .await
            .expect("Should get response");

        assert_eq!(response.status().as_u16(), 404);

        server.shutdown().await;
    }

    #[tokio::test]
    async fn test_builder_pattern() {
        use mizuchi_uploadr::metrics::server::MetricsServer;

        let mut server = MetricsServer::builder()
            .address("127.0.0.1:0")
            .build()
            .expect("Should build server");

        let addr = server.start().await.expect("Server should start");

        let client = reqwest::Client::new();
        let response = client
            .get(format!("http://{}/metrics", addr))
            .send()
            .await
            .expect("Should get metrics");

        assert!(response.status().is_success());

        server.shutdown().await;
    }
}
