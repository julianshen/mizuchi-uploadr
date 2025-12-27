//! Metrics module
//!
//! Provides Prometheus metrics and OpenTelemetry tracing.

pub mod server;

use lazy_static::lazy_static;
use prometheus::{
    register_counter, register_counter_vec, register_histogram, register_histogram_vec, Counter,
    CounterVec, Histogram, HistogramVec,
};

lazy_static! {
    // Upload metrics
    pub static ref UPLOADS_TOTAL: CounterVec = register_counter_vec!(
        "mizuchi_uploads_total",
        "Total number of uploads",
        &["bucket", "status"]
    ).unwrap();

    pub static ref UPLOAD_BYTES_TOTAL: Counter = register_counter!(
        "mizuchi_upload_bytes_total",
        "Total bytes uploaded"
    ).unwrap();

    pub static ref UPLOAD_DURATION: HistogramVec = register_histogram_vec!(
        "mizuchi_upload_duration_seconds",
        "Upload duration in seconds",
        &["bucket", "method"],
        vec![0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0, 5.0, 10.0]
    ).unwrap();

    // Zero-copy metrics
    pub static ref ZERO_COPY_BYTES: Counter = register_counter!(
        "mizuchi_zero_copy_bytes_total",
        "Bytes transferred via zero-copy (splice/sendfile)"
    ).unwrap();

    pub static ref ZERO_COPY_TRANSFERS: CounterVec = register_counter_vec!(
        "mizuchi_zero_copy_transfers_total",
        "Number of transfers by mode",
        &["mode"]  // "zero_copy" or "buffered"
    ).unwrap();

    pub static ref ZERO_COPY_DURATION: HistogramVec = register_histogram_vec!(
        "mizuchi_zero_copy_duration_seconds",
        "Zero-copy transfer duration in seconds",
        &["mode"],
        vec![0.0001, 0.0005, 0.001, 0.005, 0.01, 0.05, 0.1, 0.5, 1.0]
    ).unwrap();

    // Multipart metrics
    pub static ref MULTIPART_UPLOADS: CounterVec = register_counter_vec!(
        "mizuchi_multipart_uploads_total",
        "Total multipart uploads",
        &["bucket", "status"]
    ).unwrap();

    pub static ref MULTIPART_PARTS: Histogram = register_histogram!(
        "mizuchi_multipart_parts",
        "Number of parts per multipart upload",
        vec![1.0, 5.0, 10.0, 50.0, 100.0, 500.0, 1000.0]
    ).unwrap();

    // Auth metrics
    pub static ref AUTH_ATTEMPTS: CounterVec = register_counter_vec!(
        "mizuchi_auth_attempts_total",
        "Authentication attempts",
        &["method", "status"]
    ).unwrap();

    // Error metrics
    pub static ref ERRORS_TOTAL: CounterVec = register_counter_vec!(
        "mizuchi_errors_total",
        "Total errors",
        &["type"]
    ).unwrap();
}

/// Record a successful upload
pub fn record_upload_success(bucket: &str, bytes: u64) {
    UPLOADS_TOTAL.with_label_values(&[bucket, "success"]).inc();
    UPLOAD_BYTES_TOTAL.inc_by(bytes as f64);
}

/// Record a failed upload
pub fn record_upload_failure(bucket: &str) {
    UPLOADS_TOTAL.with_label_values(&[bucket, "failure"]).inc();
}

/// Record upload duration
pub fn record_upload_duration(bucket: &str, method: &str, duration_secs: f64) {
    UPLOAD_DURATION
        .with_label_values(&[bucket, method])
        .observe(duration_secs);
}

/// Record a data transfer with mode tracking
///
/// # Arguments
/// * `bytes` - Number of bytes transferred
/// * `duration_secs` - Transfer duration in seconds
/// * `zero_copy` - Whether zero-copy (splice/sendfile) was used
pub fn record_data_transfer(bytes: u64, duration_secs: f64, zero_copy: bool) {
    let mode = if zero_copy { "zero_copy" } else { "buffered" };
    ZERO_COPY_TRANSFERS.with_label_values(&[mode]).inc();
    ZERO_COPY_DURATION.with_label_values(&[mode]).observe(duration_secs);

    if zero_copy {
        ZERO_COPY_BYTES.inc_by(bytes as f64);
    }
}

/// Record zero-copy transfer (legacy compatibility)
pub fn record_zero_copy_transfer(bytes: u64) {
    ZERO_COPY_BYTES.inc_by(bytes as f64);
    ZERO_COPY_TRANSFERS.with_label_values(&["zero_copy"]).inc();
}

/// Record authentication attempt
pub fn record_auth_attempt(method: &str, success: bool) {
    let status = if success { "success" } else { "failure" };
    AUTH_ATTEMPTS.with_label_values(&[method, status]).inc();
}

/// Record an error
pub fn record_error(error_type: &str) {
    ERRORS_TOTAL.with_label_values(&[error_type]).inc();
}

/// Record a successful multipart upload
pub fn record_multipart_upload_success(bucket: &str, parts_count: usize) {
    MULTIPART_UPLOADS
        .with_label_values(&[bucket, "success"])
        .inc();
    MULTIPART_PARTS.observe(parts_count as f64);
}

/// Record a failed multipart upload
pub fn record_multipart_upload_failure(bucket: &str) {
    MULTIPART_UPLOADS
        .with_label_values(&[bucket, "failure"])
        .inc();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_upload_success() {
        record_upload_success("test-bucket", 1024);
        // Just verify it doesn't panic
    }

    #[test]
    fn test_record_zero_copy() {
        record_zero_copy_transfer(65536);
        // Just verify it doesn't panic
    }

    #[test]
    fn test_record_data_transfer_zero_copy() {
        record_data_transfer(1024 * 1024, 0.005, true);
        // Just verify it doesn't panic
    }

    #[test]
    fn test_record_data_transfer_buffered() {
        record_data_transfer(1024 * 1024, 0.05, false);
        // Just verify it doesn't panic
    }

    #[test]
    fn test_record_multipart_upload_success() {
        record_multipart_upload_success("test-bucket", 5);
        // Just verify it doesn't panic
    }

    #[test]
    fn test_record_multipart_upload_failure() {
        record_multipart_upload_failure("test-bucket");
        // Just verify it doesn't panic
    }
}
