//! Grafana Tempo Tracing Example
//!
//! This example demonstrates how to configure Mizuchi Uploadr with Grafana Tempo for distributed tracing.
//!
//! # Prerequisites
//!
//! 1. Create a `tempo.yaml` configuration file:
//!    ```yaml
//!    server:
//!      http_listen_port: 3200
//!    
//!    distributor:
//!      receivers:
//!        otlp:
//!          protocols:
//!            grpc:
//!              endpoint: 0.0.0.0:4317
//!    
//!    storage:
//!      trace:
//!        backend: local
//!        local:
//!          path: /tmp/tempo/traces
//!    ```
//!
//! 2. Start Tempo:
//!    ```bash
//!    docker run -d --name tempo \
//!      -p 4317:4317 \
//!      -p 3200:3200 \
//!      -v $(pwd)/tempo.yaml:/etc/tempo.yaml \
//!      grafana/tempo:latest \
//!      -config.file=/etc/tempo.yaml
//!    ```
//!
//! 3. Run this example:
//!    ```bash
//!    cargo run --example tracing_tempo --features tracing
//!    ```
//!
//! 4. Query traces via Tempo API:
//!    ```bash
//!    curl http://localhost:3200/api/search
//!    ```
//!

#[cfg(feature = "tracing")]
use mizuchi_uploadr::config::{BatchConfig, OtlpConfig, SamplingConfig, TracingConfig};
#[cfg(feature = "tracing")]
use mizuchi_uploadr::tracing::init::init_tracing;

#[cfg(feature = "tracing")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🚀 Mizuchi Uploadr - Grafana Tempo Tracing Example");
    println!("===================================================\n");

    // Configure tracing for Grafana Tempo
    let tracing_config = TracingConfig {
        enabled: true,
        service_name: "mizuchi-uploadr-tempo".to_string(),
        otlp: OtlpConfig {
            endpoint: "http://localhost:4317".to_string(),
            protocol: "grpc".to_string(),
            timeout_seconds: 10,
            compression: Some("gzip".to_string()), // Tempo supports gzip compression
        },
        sampling: SamplingConfig {
            strategy: "ratio".to_string(), // Sample 50% for demo
            ratio: 0.5,
        },
        batch: BatchConfig {
            max_queue_size: 4096, // Larger queue for production-like setup
            scheduled_delay_millis: 5000, // Export every 5 seconds
            max_export_batch_size: 1024,
        },
    };

    println!("📊 Initializing tracing...");
    println!("   Endpoint: {}", tracing_config.otlp.endpoint);
    println!("   Service: {}", tracing_config.service_name);
    println!("   Sampling: {} ({})", tracing_config.sampling.strategy, tracing_config.sampling.ratio);
    println!("   Compression: {:?}", tracing_config.otlp.compression);

    // Initialize tracing
    let _tracing_guard = init_tracing(&tracing_config)?;
    println!("✅ Tracing initialized\n");

    // Simulate a multi-step workflow
    println!("🔄 Simulating multi-step workflow...");
    simulate_workflow().await?;

    println!("\n✅ Workflow complete!");
    println!("\n📊 Query traces via Tempo API:");
    println!("   curl http://localhost:3200/api/search");
    println!("   curl http://localhost:3200/api/traces/<trace-id>");

    // Give time for spans to be exported
    println!("\n⏳ Waiting for spans to be exported...");
    tokio::time::sleep(tokio::time::Duration::from_secs(6)).await;

    println!("✅ Done!");

    Ok(())
}

#[cfg(feature = "tracing")]
#[tracing::instrument(name = "workflow")]
async fn simulate_workflow() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Starting workflow");

    // Step 1: Validate request
    validate_request().await?;

    // Step 2: Process upload
    process_upload().await?;

    // Step 3: Notify completion
    notify_completion().await?;

    tracing::info!("Workflow complete");
    Ok(())
}

#[cfg(feature = "tracing")]
#[tracing::instrument(name = "workflow.validate")]
async fn validate_request() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Validating request");

    // Simulate validation steps
    {
        let _span = tracing::info_span!("validate.auth").entered();
        tracing::info!("Checking authentication");
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    {
        let _span = tracing::info_span!("validate.authz").entered();
        tracing::info!("Checking authorization");
        tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;
    }

    {
        let _span = tracing::info_span!("validate.quota").entered();
        tracing::info!("Checking quota");
        tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;
    }

    tracing::info!("Validation complete");
    println!("   ✅ Request validated");
    Ok(())
}

#[cfg(feature = "tracing")]
#[tracing::instrument(name = "workflow.upload")]
async fn process_upload() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!(file_size = 5242880, "Processing upload");

    // Simulate upload steps
    {
        let _span = tracing::info_span!("upload.prepare", bucket = "my-bucket").entered();
        tracing::info!("Preparing upload");
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    {
        let _span = tracing::info_span!("upload.transfer", zero_copy = true).entered();
        tracing::info!("Transferring data");
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    }

    {
        let _span = tracing::info_span!("upload.verify").entered();
        tracing::info!("Verifying upload");
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
    }

    tracing::info!("Upload complete");
    println!("   ✅ Upload processed");
    Ok(())
}

#[cfg(feature = "tracing")]
#[tracing::instrument(name = "workflow.notify")]
async fn notify_completion() -> Result<(), Box<dyn std::error::Error>> {
    tracing::info!("Sending completion notification");

    tokio::time::sleep(tokio::time::Duration::from_millis(30)).await;

    tracing::info!("Notification sent");
    println!("   ✅ Notification sent");
    Ok(())
}

#[cfg(not(feature = "tracing"))]
fn main() {
    eprintln!("This example requires the 'tracing' feature.");
    eprintln!("Run with: cargo run --example tracing_tempo --features tracing");
    std::process::exit(1);
}

