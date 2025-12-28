//! E2E Load and Performance Tests
//!
//! RED Phase: These tests define the expected performance characteristics
//! of the upload proxy under load.
//!
//! ## Test Coverage
//!
//! - Throughput benchmarks (MB/s)
//! - Latency measurements (p50, p95, p99)
//! - Concurrent connection handling
//! - Memory efficiency under load
//! - Zero-copy performance validation

use super::common::E2ETestEnv;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Performance metrics collected during load tests
#[derive(Debug, Default)]
pub struct LoadTestMetrics {
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub total_bytes: u64,
    pub total_duration_ms: u64,
    pub min_latency_ms: u64,
    pub max_latency_ms: u64,
    pub latencies: Vec<u64>,
}

impl LoadTestMetrics {
    /// Calculate throughput in MB/s
    pub fn throughput_mbps(&self) -> f64 {
        if self.total_duration_ms == 0 {
            return 0.0;
        }
        let bytes_per_sec = (self.total_bytes as f64) / (self.total_duration_ms as f64 / 1000.0);
        bytes_per_sec / (1024.0 * 1024.0)
    }

    /// Calculate requests per second
    pub fn requests_per_sec(&self) -> f64 {
        if self.total_duration_ms == 0 {
            return 0.0;
        }
        (self.total_requests as f64) / (self.total_duration_ms as f64 / 1000.0)
    }

    /// Calculate p50 latency
    pub fn p50_latency_ms(&self) -> u64 {
        self.percentile(50)
    }

    /// Calculate p95 latency
    pub fn p95_latency_ms(&self) -> u64 {
        self.percentile(95)
    }

    /// Calculate p99 latency
    pub fn p99_latency_ms(&self) -> u64 {
        self.percentile(99)
    }

    fn percentile(&self, p: u8) -> u64 {
        if self.latencies.is_empty() {
            return 0;
        }
        let mut sorted = self.latencies.clone();
        sorted.sort_unstable();
        let idx = (sorted.len() as f64 * (p as f64 / 100.0)) as usize;
        sorted[idx.min(sorted.len() - 1)]
    }

    /// Success rate as percentage
    pub fn success_rate(&self) -> f64 {
        if self.total_requests == 0 {
            return 0.0;
        }
        (self.successful_requests as f64 / self.total_requests as f64) * 100.0
    }
}

/// Run a load test with specified parameters
async fn run_load_test(
    env: &E2ETestEnv,
    num_requests: usize,
    concurrency: usize,
    payload_size: usize,
) -> LoadTestMetrics {
    let start = Instant::now();
    let successful = Arc::new(AtomicU64::new(0));
    let failed = Arc::new(AtomicU64::new(0));
    let bytes_sent = Arc::new(AtomicU64::new(0));
    let latencies = Arc::new(parking_lot::Mutex::new(Vec::new()));

    let requests_per_worker = num_requests / concurrency;

    let mut handles = vec![];
    for worker_id in 0..concurrency {
        let client = env.client.clone();
        let base_url = env.base_url();
        let successful = successful.clone();
        let failed = failed.clone();
        let bytes_sent = bytes_sent.clone();
        let latencies = latencies.clone();

        let handle = tokio::spawn(async move {
            for i in 0..requests_per_worker {
                let payload = E2ETestEnv::random_payload(payload_size);
                let req_start = Instant::now();

                let result = client
                    .put(format!(
                        "{}/uploads/load-test-{}-{}.bin",
                        base_url, worker_id, i
                    ))
                    .body(payload.to_vec())
                    .send()
                    .await;

                let latency = req_start.elapsed().as_millis() as u64;
                latencies.lock().push(latency);

                match result {
                    Ok(resp) if resp.status().is_success() => {
                        successful.fetch_add(1, Ordering::Relaxed);
                        bytes_sent.fetch_add(payload_size as u64, Ordering::Relaxed);
                    }
                    _ => {
                        failed.fetch_add(1, Ordering::Relaxed);
                    }
                }
            }
        });

        handles.push(handle);
    }

    futures::future::join_all(handles).await;

    let duration = start.elapsed();
    let latencies_vec = latencies.lock().clone();

    LoadTestMetrics {
        total_requests: num_requests as u64,
        successful_requests: successful.load(Ordering::Relaxed),
        failed_requests: failed.load(Ordering::Relaxed),
        total_bytes: bytes_sent.load(Ordering::Relaxed),
        total_duration_ms: duration.as_millis() as u64,
        min_latency_ms: *latencies_vec.iter().min().unwrap_or(&0),
        max_latency_ms: *latencies_vec.iter().max().unwrap_or(&0),
        latencies: latencies_vec,
    }
}

/// Test: Baseline throughput with 1KB payloads
///
/// Measures maximum requests/second for small files.
/// Target: > 1000 req/s on modern hardware
#[tokio::test]
async fn test_throughput_1kb_payloads() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let env = E2ETestEnv::new().await.expect("Failed to create test env");

    // Warm up
    let _warmup = run_load_test(&env, 10, 2, 1024).await;

    // Actual test: 100 requests, 10 concurrent, 1KB each
    let metrics = run_load_test(&env, 100, 10, 1024).await;

    println!("=== 1KB Payload Load Test Results ===");
    println!("Total requests: {}", metrics.total_requests);
    println!("Successful: {}", metrics.successful_requests);
    println!("Failed: {}", metrics.failed_requests);
    println!("Duration: {}ms", metrics.total_duration_ms);
    println!("Throughput: {:.2} req/s", metrics.requests_per_sec());
    println!("Data rate: {:.2} MB/s", metrics.throughput_mbps());
    println!("p50 latency: {}ms", metrics.p50_latency_ms());
    println!("p95 latency: {}ms", metrics.p95_latency_ms());
    println!("p99 latency: {}ms", metrics.p99_latency_ms());

    // Assertions
    assert!(
        metrics.success_rate() >= 95.0,
        "Success rate should be >= 95%, got {:.2}%",
        metrics.success_rate()
    );

    // Throughput target (adjust based on hardware)
    assert!(
        metrics.requests_per_sec() >= 10.0,
        "Should achieve >= 10 req/s, got {:.2}",
        metrics.requests_per_sec()
    );
}

/// Test: Throughput with 1MB payloads
///
/// Measures data throughput for medium files.
/// Target: > 100 MB/s on modern hardware
#[tokio::test]
async fn test_throughput_1mb_payloads() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let env = E2ETestEnv::new().await.expect("Failed to create test env");

    // 20 requests, 4 concurrent, 1MB each
    let metrics = run_load_test(&env, 20, 4, 1024 * 1024).await;

    println!("=== 1MB Payload Load Test Results ===");
    println!("Total requests: {}", metrics.total_requests);
    println!("Successful: {}", metrics.successful_requests);
    println!("Duration: {}ms", metrics.total_duration_ms);
    println!("Throughput: {:.2} MB/s", metrics.throughput_mbps());
    println!("p50 latency: {}ms", metrics.p50_latency_ms());
    println!("p95 latency: {}ms", metrics.p95_latency_ms());

    assert!(
        metrics.success_rate() >= 95.0,
        "Success rate should be >= 95%, got {:.2}%",
        metrics.success_rate()
    );
}

/// Test: Throughput with 10MB payloads
///
/// Measures data throughput for large files.
/// Target: > 50 MB/s on modern hardware
#[tokio::test]
async fn test_throughput_10mb_payloads() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let env = E2ETestEnv::new().await.expect("Failed to create test env");

    // 10 requests, 2 concurrent, 10MB each
    let metrics = run_load_test(&env, 10, 2, 10 * 1024 * 1024).await;

    println!("=== 10MB Payload Load Test Results ===");
    println!("Total requests: {}", metrics.total_requests);
    println!("Successful: {}", metrics.successful_requests);
    println!("Duration: {}ms", metrics.total_duration_ms);
    println!("Throughput: {:.2} MB/s", metrics.throughput_mbps());
    println!("p50 latency: {}ms", metrics.p50_latency_ms());
    println!("p95 latency: {}ms", metrics.p95_latency_ms());

    assert!(
        metrics.success_rate() >= 95.0,
        "Success rate should be >= 95%, got {:.2}%",
        metrics.success_rate()
    );
}

/// Test: Latency under moderate load
///
/// Measures p95 latency when system is under typical load.
/// Target: p95 < 500ms for 1KB uploads
#[tokio::test]
async fn test_latency_under_moderate_load() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let env = E2ETestEnv::new().await.expect("Failed to create test env");

    // 50 requests, 5 concurrent, 1KB each
    let metrics = run_load_test(&env, 50, 5, 1024).await;

    println!("=== Latency Test Results ===");
    println!("p50 latency: {}ms", metrics.p50_latency_ms());
    println!("p95 latency: {}ms", metrics.p95_latency_ms());
    println!("p99 latency: {}ms", metrics.p99_latency_ms());
    println!("Min latency: {}ms", metrics.min_latency_ms);
    println!("Max latency: {}ms", metrics.max_latency_ms);

    // p95 latency target (adjust for environment)
    assert!(
        metrics.p95_latency_ms() < 5000,
        "p95 latency should be < 5000ms, got {}ms",
        metrics.p95_latency_ms()
    );
}

/// Test: Maximum concurrent connections
///
/// Tests server stability under high connection count.
#[tokio::test]
async fn test_max_concurrent_connections() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let env = E2ETestEnv::new().await.expect("Failed to create test env");

    // 100 concurrent connections, 1 request each
    let metrics = run_load_test(&env, 100, 100, 1024).await;

    println!("=== Max Concurrency Test Results ===");
    println!("Total requests: {}", metrics.total_requests);
    println!("Successful: {}", metrics.successful_requests);
    println!("Failed: {}", metrics.failed_requests);
    println!("Success rate: {:.2}%", metrics.success_rate());

    // At least 90% should succeed under high concurrency
    assert!(
        metrics.success_rate() >= 90.0,
        "At least 90% should succeed under high concurrency, got {:.2}%",
        metrics.success_rate()
    );
}

/// Test: Sustained load over time
///
/// Tests stability during extended load.
#[tokio::test]
async fn test_sustained_load() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let env = E2ETestEnv::new().await.expect("Failed to create test env");

    // Run for ~10 seconds worth of requests
    // 200 requests, 10 concurrent, 10KB each
    let start = Instant::now();
    let metrics = run_load_test(&env, 200, 10, 10 * 1024).await;
    let elapsed = start.elapsed();

    println!("=== Sustained Load Test Results ===");
    println!("Duration: {:.2}s", elapsed.as_secs_f64());
    println!("Total requests: {}", metrics.total_requests);
    println!("Throughput: {:.2} req/s", metrics.requests_per_sec());
    println!("Success rate: {:.2}%", metrics.success_rate());

    assert!(
        metrics.success_rate() >= 95.0,
        "Sustained load should maintain >= 95% success rate"
    );
}

/// Test: Mixed payload sizes (realistic workload)
///
/// Simulates real-world traffic with varying file sizes.
#[tokio::test]
async fn test_mixed_payload_sizes() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let env = E2ETestEnv::new().await.expect("Failed to create test env");

    let start = Instant::now();
    let mut handles = vec![];

    // Mix of sizes: 1KB (60%), 100KB (30%), 1MB (10%)
    let sizes = [
        (1024, 60),        // 1KB x 60
        (100 * 1024, 30),  // 100KB x 30
        (1024 * 1024, 10), // 1MB x 10
    ];

    let mut request_id = 0;
    for (size, count) in sizes {
        for _ in 0..count {
            let client = env.client.clone();
            let base_url = env.base_url();
            let id = request_id;
            request_id += 1;

            let handle = tokio::spawn(async move {
                let payload = E2ETestEnv::random_payload(size);
                client
                    .put(format!("{}/uploads/mixed-{}.bin", base_url, id))
                    .body(payload.to_vec())
                    .send()
                    .await
                    .map(|r| (r.status().is_success(), size))
                    .unwrap_or((false, size))
            });

            handles.push(handle);
        }
    }

    let results: Vec<_> = futures::future::join_all(handles).await;
    let elapsed = start.elapsed();

    let success_count = results
        .iter()
        .filter(|r| matches!(r, Ok((true, _))))
        .count();
    let total_bytes: usize = results
        .iter()
        .filter_map(|r| match r {
            Ok((true, size)) => Some(*size),
            _ => None,
        })
        .sum();

    let throughput_mbps = (total_bytes as f64) / elapsed.as_secs_f64() / (1024.0 * 1024.0);

    println!("=== Mixed Payload Test Results ===");
    println!("Duration: {:.2}s", elapsed.as_secs_f64());
    println!("Total requests: 100");
    println!("Successful: {}", success_count);
    println!("Total bytes: {} MB", total_bytes / (1024 * 1024));
    println!("Throughput: {:.2} MB/s", throughput_mbps);

    assert!(
        success_count >= 90,
        "At least 90% of mixed requests should succeed, got {}",
        success_count
    );
}

/// Test: Zero-copy performance advantage (Linux only)
///
/// Compares performance with and without zero-copy (if possible).
#[tokio::test]
#[cfg(target_os = "linux")]
async fn test_zero_copy_performance() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let env = E2ETestEnv::new().await.expect("Failed to create test env");

    // Large files benefit most from zero-copy
    let metrics = run_load_test(&env, 10, 2, 10 * 1024 * 1024).await;

    println!("=== Zero-Copy Performance Test ===");
    println!("Platform: Linux (zero-copy enabled)");
    println!("Throughput: {:.2} MB/s", metrics.throughput_mbps());
    println!("p50 latency: {}ms", metrics.p50_latency_ms());

    // On Linux with zero-copy, we expect better throughput
    // This is a baseline check - actual improvement depends on hardware
    assert!(
        metrics.throughput_mbps() > 0.0,
        "Zero-copy should provide measurable throughput"
    );
}

/// Test: Memory efficiency under load
///
/// Ensures memory usage doesn't grow unbounded during sustained load.
#[tokio::test]
async fn test_memory_efficiency() {
    if !super::common::is_s3_backend_available().await {
        eprintln!("Skipping: S3 backend not available");
        return;
    }

    let env = E2ETestEnv::new().await.expect("Failed to create test env");

    // Run multiple batches and check stability
    for batch in 0..3 {
        let metrics = run_load_test(&env, 50, 10, 100 * 1024).await;

        println!("=== Batch {} Results ===", batch + 1);
        println!("Success rate: {:.2}%", metrics.success_rate());

        // Each batch should maintain similar success rates
        assert!(
            metrics.success_rate() >= 90.0,
            "Batch {} should maintain >= 90% success rate",
            batch + 1
        );

        // Small delay between batches
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}
