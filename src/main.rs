//! Mizuchi Uploadr - High-performance upload-only S3 proxy
//!
//! A secure, zero-copy S3 proxy that only allows upload operations.

use clap::Parser;
use mizuchi_uploadr::{config::Config, server::Server};
use std::path::PathBuf;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

/// Mizuchi Uploadr - Upload-only S3 proxy with zero-copy optimization
#[derive(Parser, Debug)]
#[command(name = "mizuchi-uploadr")]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, default_value = "config.yaml")]
    config: PathBuf,

    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Initialize logging
    let level = match args.log_level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        _ => Level::INFO,
    };

    let subscriber = FmtSubscriber::builder()
        .with_max_level(level)
        .with_target(true)
        .with_thread_ids(true)
        .json()
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    info!("Starting Mizuchi Uploadr v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config = Config::load(&args.config)?;
    info!("Loaded configuration from {:?}", args.config);

    // Start server
    let server = Server::new(config)?;
    server.run().await?;

    Ok(())
}
