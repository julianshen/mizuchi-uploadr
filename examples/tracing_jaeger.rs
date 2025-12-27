//! Jaeger Tracing Example
//!
//! This example demonstrates how to configure Mizuchi Uploadr with Jaeger for distributed tracing.
//!
//! # Prerequisites
//!
//! 1. Start Jaeger:
//!    ```bash
//!    docker run -d --name jaeger \
//!      -p 4317:4317 \
//!      -p 16686:16686 \
//!      jaegertracing/all-in-one:latest
//!    ```
//!
//! 2. Run this example:
//!    ```bash
//!    cargo run --example tracing_jaeger --features tracing
//!    ```
//!
//! 3. View traces in Jaeger UI:
//!    http://localhost:16686
//!

#[cfg(feature = "tracing")]
use mizuchi_uploadr::auth::jwt::JwtAuthenticator;
#[cfg(feature = "tracing")]
use mizuchi_uploadr::auth::{AuthRequest, Authenticator};
#[cfg(feature = "tracing")]
use mizuchi_uploadr::authz::opa::{OpaAuthorizer, OpaConfig};
#[cfg(feature = "tracing")]
use mizuchi_uploadr::authz::{Authorizer, AuthzRequest};
#[cfg(feature = "tracing")]
use mizuchi_uploadr::config::{BatchConfig, OtlpConfig, SamplingConfig, TracingConfig};
#[cfg(feature = "tracing")]
use mizuchi_uploadr::tracing::init::init_tracing;
#[cfg(feature = "tracing")]
use std::collections::HashMap;

#[cfg(feature = "tracing")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Mizuchi Uploadr - Jaeger Tracing Example");
    println!("============================================\n");

    // Configure tracing for Jaeger
    let tracing_config = TracingConfig {
        enabled: true,
        service_name: "mizuchi-uploadr-example".to_string(),
        otlp: OtlpConfig {
            endpoint: "http://localhost:4317".to_string(),
            protocol: "grpc".to_string(),
            timeout_seconds: 10,
            compression: None,
        },
        sampling: SamplingConfig {
            strategy: "always".to_string(), // Sample all traces for demo
            ratio: 1.0,
        },
        batch: BatchConfig {
            max_queue_size: 2048,
            scheduled_delay_millis: 1000, // Export every second for quick feedback
            max_export_batch_size: 512,
        },
    };

    println!("📊 Initializing tracing...");
    println!("   Endpoint: {}", tracing_config.otlp.endpoint);
    println!("   Service: {}", tracing_config.service_name);
    println!(
        "   Sampling: {} ({})",
        tracing_config.sampling.strategy, tracing_config.sampling.ratio
    );

    // Initialize tracing (guard will flush spans on drop)
    let _tracing_guard = init_tracing(&tracing_config)?;
    println!("✅ Tracing initialized\n");

    // Simulate some traced operations
    println!("🔐 Simulating authentication...");
    simulate_authentication().await?;

    println!("\n🛡️  Simulating authorization...");
    simulate_authorization().await?;

    println!("\n📤 Simulating upload operation...");
    simulate_upload().await?;

    println!("\n✅ All operations complete!");
    println!("\n📊 View traces in Jaeger UI:");
    println!("   http://localhost:16686");
    println!("   Service: mizuchi-uploadr-example");

    // Give time for spans to be exported
    println!("\n⏳ Waiting for spans to be exported...");
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    println!("✅ Done! Check Jaeger UI for traces.");

    Ok(())
    // Guard drops here, flushing all remaining spans
}

#[cfg(feature = "tracing")]
async fn simulate_authentication() -> Result<(), Box<dyn std::error::Error>> {
    // Create a JWT authenticator
    let authenticator = JwtAuthenticator::new_hs256("secret-key-for-demo");

    // Create a mock auth request
    let mut headers = HashMap::new();
    headers.insert(
        "authorization".to_string(),
        "Bearer eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJzdWIiOiJ1c2VyMTIzIiwiZXhwIjo5OTk5OTk5OTk5fQ.demo".to_string(),
    );

    let request = AuthRequest {
        headers,
        query: None,
        method: "PUT".to_string(),
        path: "/uploads/file.txt".to_string(),
    };

    // This will create an "auth.jwt" span
    match authenticator.authenticate(&request).await {
        Ok(result) => println!("   ✅ Authenticated: {}", result.subject),
        Err(e) => println!("   ❌ Authentication failed: {}", e),
    }

    Ok(())
}

#[cfg(feature = "tracing")]
async fn simulate_authorization() -> Result<(), Box<dyn std::error::Error>> {
    // Create an OPA authorizer (won't actually connect in this demo)
    let authorizer = OpaAuthorizer::new(OpaConfig {
        url: "http://localhost:8181".to_string(),
        policy_path: "mizuchi/allow".to_string(),
        timeout: None,
        cache_ttl: None,
    });

    // Create a mock authz request
    let request = AuthzRequest {
        subject: "user123".to_string(),
        action: "upload".to_string(),
        resource: "bucket/my-uploads".to_string(),
        context: HashMap::new(),
    };

    // This will create an "authz.opa" span
    // Note: This will fail because OPA is not running, but the span will still be created
    match authorizer.authorize(&request).await {
        Ok(allowed) => println!(
            "   ✅ Authorization: {}",
            if allowed { "allowed" } else { "denied" }
        ),
        Err(e) => println!("   ⚠️  Authorization check failed (expected): {}", e),
    }

    Ok(())
}

#[cfg(feature = "tracing")]
#[tracing::instrument(name = "upload.simulate")]
async fn simulate_upload() -> Result<(), Box<dyn std::error::Error>> {
    // Simulate upload steps with manual spans
    tracing::info!("Starting upload simulation");

    // Simulate file validation
    {
        let _span = tracing::info_span!("upload.validate").entered();
        tracing::info!(file_size = 1024000, "Validating file");
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    // Simulate S3 upload
    {
        let _span =
            tracing::info_span!("s3.put_object", bucket = "my-bucket", key = "file.txt").entered();
        tracing::info!("Uploading to S3");
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    tracing::info!("Upload complete");
    println!("   ✅ Upload simulated");

    Ok(())
}

#[cfg(not(feature = "tracing"))]
fn main() {
    eprintln!("This example requires the 'tracing' feature.");
    eprintln!("Run with: cargo run --example tracing_jaeger --features tracing");
    std::process::exit(1);
}
