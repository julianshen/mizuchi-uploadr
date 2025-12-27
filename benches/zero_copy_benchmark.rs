//! Zero-copy vs Buffered I/O Benchmarks
//!
//! Compares performance of:
//! - Direct Bytes (in-memory) vs TempFileUpload (file-backed)
//! - SHA256 hash computation
//! - Data transfer overhead
//!
//! Run with: cargo bench --bench zero_copy_benchmark

use bytes::Bytes;
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use mizuchi_uploadr::upload::temp_file::TempFileUpload;
use mizuchi_uploadr::upload::zero_copy::{DataTransfer, DEFAULT_BUFFER_SIZE};
use sha2::{Digest, Sha256};
use std::io::Read;

// ============================================================================
// Benchmark: Zero-Copy Availability Check
// ============================================================================

fn benchmark_zero_copy_available(c: &mut Criterion) {
    c.bench_function("zero_copy_available_check", |b| {
        b.iter(|| {
            black_box(mizuchi_uploadr::zero_copy_available());
        });
    });
}

fn benchmark_data_transfer_creation(c: &mut Criterion) {
    c.bench_function("data_transfer_creation", |b| {
        b.iter(|| {
            let transfer = DataTransfer::new(DEFAULT_BUFFER_SIZE, true);
            let _ = black_box(transfer);
        });
    });
}

// ============================================================================
// Benchmark: Bytes vs TempFileUpload Creation
// ============================================================================

fn benchmark_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("creation");

    // Test sizes: 1KB, 10KB, 100KB, 1MB, 5MB, 10MB
    let sizes: Vec<usize> = vec![
        1024,           // 1 KB
        10 * 1024,      // 10 KB
        100 * 1024,     // 100 KB
        1024 * 1024,    // 1 MB
        5 * 1024 * 1024, // 5 MB
        10 * 1024 * 1024, // 10 MB
    ];

    for size in sizes {
        let data = vec![0xABu8; size];

        group.throughput(Throughput::Bytes(size as u64));

        // Benchmark: Create Bytes directly
        group.bench_with_input(
            BenchmarkId::new("bytes_direct", format_size(size)),
            &data,
            |b, data| {
                b.iter(|| {
                    let bytes = Bytes::from(data.clone());
                    black_box(bytes);
                });
            },
        );

        // Benchmark: Create TempFileUpload (includes file write + hash)
        group.bench_with_input(
            BenchmarkId::new("temp_file_upload", format_size(size)),
            &data,
            |b, data| {
                b.iter(|| {
                    let bytes = Bytes::from(data.clone());
                    let temp = TempFileUpload::from_bytes(bytes).unwrap();
                    black_box(temp);
                    // temp is dropped here, file is cleaned up
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Benchmark: SHA256 Hash Computation
// ============================================================================

fn benchmark_hash_computation(c: &mut Criterion) {
    let mut group = c.benchmark_group("hash_computation");

    let sizes: Vec<usize> = vec![
        1024 * 1024,      // 1 MB
        5 * 1024 * 1024,  // 5 MB
        10 * 1024 * 1024, // 10 MB
    ];

    for size in sizes {
        let data = vec![0xCDu8; size];
        let bytes = Bytes::from(data.clone());

        group.throughput(Throughput::Bytes(size as u64));

        // Benchmark: Hash in-memory Bytes
        group.bench_with_input(
            BenchmarkId::new("hash_bytes_memory", format_size(size)),
            &bytes,
            |b, bytes| {
                b.iter(|| {
                    let mut hasher = Sha256::new();
                    hasher.update(bytes.as_ref());
                    let hash = hex::encode(hasher.finalize());
                    black_box(hash);
                });
            },
        );

        // Benchmark: Hash from file (read + hash)
        // Create temp file once, then benchmark reading from it
        let temp = TempFileUpload::from_bytes(bytes.clone()).unwrap();
        let path = temp.path().to_path_buf();

        group.bench_with_input(
            BenchmarkId::new("hash_from_file", format_size(size)),
            &path,
            |b, path| {
                b.iter(|| {
                    let mut file = std::fs::File::open(path).unwrap();
                    let mut buffer = Vec::new();
                    file.read_to_end(&mut buffer).unwrap();

                    let mut hasher = Sha256::new();
                    hasher.update(&buffer);
                    let hash = hex::encode(hasher.finalize());
                    black_box(hash);
                });
            },
        );

        // Benchmark: Pre-computed hash access (just retrieve stored hash)
        group.bench_with_input(
            BenchmarkId::new("hash_precomputed", format_size(size)),
            &temp,
            |b, temp| {
                b.iter(|| {
                    let hash = temp.content_hash();
                    black_box(hash);
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Benchmark: Data Read Performance
// ============================================================================

fn benchmark_data_read(c: &mut Criterion) {
    let mut group = c.benchmark_group("data_read");

    let sizes: Vec<usize> = vec![
        1024 * 1024,      // 1 MB
        5 * 1024 * 1024,  // 5 MB
        10 * 1024 * 1024, // 10 MB
    ];

    for size in sizes {
        let data = vec![0xEFu8; size];
        let bytes = Bytes::from(data);

        group.throughput(Throughput::Bytes(size as u64));

        // Benchmark: Clone Bytes (what happens in current put_object)
        group.bench_with_input(
            BenchmarkId::new("bytes_clone", format_size(size)),
            &bytes,
            |b, bytes| {
                b.iter(|| {
                    let cloned = bytes.clone();
                    black_box(cloned);
                });
            },
        );

        // Benchmark: Read from temp file
        let temp = TempFileUpload::from_bytes(bytes.clone()).unwrap();
        let path = temp.path().to_path_buf();

        group.bench_with_input(
            BenchmarkId::new("file_read", format_size(size)),
            &path,
            |b, path| {
                b.iter(|| {
                    let mut file = std::fs::File::open(path).unwrap();
                    let mut buffer = Vec::with_capacity(size);
                    file.read_to_end(&mut buffer).unwrap();
                    black_box(buffer);
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Benchmark: Full Upload Path Simulation
// ============================================================================

fn benchmark_upload_path(c: &mut Criterion) {
    let mut group = c.benchmark_group("upload_path");

    let sizes: Vec<usize> = vec![
        1024 * 1024,      // 1 MB
        5 * 1024 * 1024,  // 5 MB
        10 * 1024 * 1024, // 10 MB
    ];

    for size in sizes {
        let data = vec![0x42u8; size];

        group.throughput(Throughput::Bytes(size as u64));

        // Benchmark: Direct Bytes path (current put_object behavior)
        // Steps: Create Bytes -> Hash -> "Upload" (simulate with clone)
        group.bench_with_input(
            BenchmarkId::new("buffered_path", format_size(size)),
            &data,
            |b, data| {
                b.iter(|| {
                    // 1. Create Bytes
                    let bytes = Bytes::from(data.clone());

                    // 2. Compute hash (required for SigV4)
                    let mut hasher = Sha256::new();
                    hasher.update(bytes.as_ref());
                    let hash = hex::encode(hasher.finalize());

                    // 3. Simulate upload (clone for retry support)
                    let upload_body = bytes.clone();

                    black_box((hash, upload_body));
                });
            },
        );

        // Benchmark: Temp file path (put_object_from_file behavior)
        // Steps: Create TempFileUpload (includes hash) -> Read for upload
        group.bench_with_input(
            BenchmarkId::new("tempfile_path", format_size(size)),
            &data,
            |b, data| {
                b.iter(|| {
                    // 1. Create TempFileUpload (writes to file, computes hash)
                    let bytes = Bytes::from(data.clone());
                    let temp = TempFileUpload::from_bytes(bytes).unwrap();

                    // 2. Get pre-computed hash
                    let hash = temp.content_hash().to_string();

                    // 3. Read file for upload
                    let mut file = std::fs::File::open(temp.path()).unwrap();
                    let mut upload_body = Vec::with_capacity(data.len());
                    file.read_to_end(&mut upload_body).unwrap();

                    black_box((hash, upload_body));
                    // temp dropped, file cleaned up
                });
            },
        );
    }

    group.finish();
}

// ============================================================================
// Helper Functions
// ============================================================================

fn format_size(size: usize) -> String {
    if size >= 1024 * 1024 {
        format!("{}MB", size / (1024 * 1024))
    } else if size >= 1024 {
        format!("{}KB", size / 1024)
    } else {
        format!("{}B", size)
    }
}

// ============================================================================
// Criterion Configuration
// ============================================================================

criterion_group!(
    benches,
    benchmark_zero_copy_available,
    benchmark_data_transfer_creation,
    benchmark_creation,
    benchmark_hash_computation,
    benchmark_data_read,
    benchmark_upload_path,
);

criterion_main!(benches);
