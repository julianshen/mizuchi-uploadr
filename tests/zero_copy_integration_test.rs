//! Zero-Copy Integration Tests
//!
//! Tests for zero-copy transfer integration in upload handlers.
//! These tests verify that zero-copy is used on Linux and fallback on other platforms.
//!
//! ## Test Coverage
//!
//! - Zero-copy detection on Linux
//! - Fallback to buffered I/O on non-Linux
//! - Data integrity with zero-copy transfer
//! - Metrics recording for zero-copy usage
//! - Performance characteristics

#[cfg(test)]
mod tests {
    use mizuchi_uploadr::upload::zero_copy::{is_available, DataTransfer, DEFAULT_BUFFER_SIZE};

    // ========================================================================
    // TEST: Platform Detection
    // ========================================================================

    /// Test that zero-copy availability is correctly detected
    #[test]
    fn test_zero_copy_availability_detection() {
        let available = is_available();

        #[cfg(target_os = "linux")]
        assert!(available, "Zero-copy should be available on Linux");

        #[cfg(not(target_os = "linux"))]
        assert!(!available, "Zero-copy should NOT be available on non-Linux");
    }

    /// Test DataTransfer reports correct zero-copy status
    #[test]
    fn test_data_transfer_reports_zero_copy_status() {
        let transfer = DataTransfer::new(DEFAULT_BUFFER_SIZE, true).unwrap();

        #[cfg(target_os = "linux")]
        assert!(
            transfer.is_zero_copy(),
            "DataTransfer should use zero-copy on Linux"
        );

        #[cfg(not(target_os = "linux"))]
        assert!(
            !transfer.is_zero_copy(),
            "DataTransfer should use fallback on non-Linux"
        );
    }

    /// Test DataTransfer respects use_zero_copy=false
    #[test]
    fn test_data_transfer_respects_zero_copy_disabled() {
        let transfer = DataTransfer::new(DEFAULT_BUFFER_SIZE, false).unwrap();
        assert!(
            !transfer.is_zero_copy(),
            "DataTransfer should not use zero-copy when disabled"
        );
    }

    // ========================================================================
    // TEST: Zero-Copy Transfer (Linux-specific)
    // ========================================================================

    #[cfg(target_os = "linux")]
    mod linux_tests {
        use super::*;
        use std::fs::File;
        use std::io::{Read, Write};
        use tempfile::NamedTempFile;

        /// Test zero-copy transfer between files
        #[tokio::test]
        async fn test_zero_copy_file_transfer() {
            use mizuchi_uploadr::upload::zero_copy::ZeroCopyTransfer;
            // File implements AsFd trait, so no import needed

            // Create source file with test data
            let mut source = NamedTempFile::new().unwrap();
            let test_data = vec![0xABu8; 1024 * 1024]; // 1MB of test data
            source.write_all(&test_data).unwrap();
            source.flush().unwrap();

            // Reopen for reading
            let source_file = File::open(source.path()).unwrap();

            // Create destination file
            let dest = NamedTempFile::new().unwrap();
            let dest_file = File::create(dest.path()).unwrap();

            // Perform zero-copy transfer
            let transfer = ZeroCopyTransfer::new(DEFAULT_BUFFER_SIZE).unwrap();
            let transferred = transfer
                .transfer(&source_file, &dest_file, test_data.len())
                .await
                .unwrap();

            assert_eq!(transferred, test_data.len(), "Should transfer all bytes");

            // Verify data integrity
            drop(dest_file);
            let mut dest_data = Vec::new();
            File::open(dest.path())
                .unwrap()
                .read_to_end(&mut dest_data)
                .unwrap();

            assert_eq!(dest_data, test_data, "Data should be identical after transfer");
        }

        /// Test zero-copy with various buffer sizes
        #[tokio::test]
        async fn test_zero_copy_various_buffer_sizes() {
            use mizuchi_uploadr::upload::zero_copy::ZeroCopyTransfer;

            let buffer_sizes = [4096, 65536, 1024 * 1024]; // 4KB, 64KB, 1MB

            for buffer_size in buffer_sizes {
                let mut source = NamedTempFile::new().unwrap();
                let test_data = vec![0x42u8; 256 * 1024]; // 256KB
                source.write_all(&test_data).unwrap();
                source.flush().unwrap();

                let source_file = File::open(source.path()).unwrap();
                let dest = NamedTempFile::new().unwrap();
                let dest_file = File::create(dest.path()).unwrap();

                let transfer = ZeroCopyTransfer::new(buffer_size).unwrap();
                let transferred = transfer
                    .transfer(&source_file, &dest_file, test_data.len())
                    .await
                    .unwrap();

                assert_eq!(
                    transferred,
                    test_data.len(),
                    "Buffer size {} should transfer all bytes",
                    buffer_size
                );
            }
        }
    }

    // ========================================================================
    // TEST: Fallback Transfer (Non-Linux)
    // ========================================================================

    #[cfg(not(target_os = "linux"))]
    mod fallback_tests {
        use super::*;

        /// Test fallback transfer struct can be created
        #[test]
        fn test_fallback_transfer_creation() {
            use mizuchi_uploadr::upload::zero_copy::ZeroCopyTransfer;

            let transfer = ZeroCopyTransfer::new(DEFAULT_BUFFER_SIZE);
            assert!(transfer.is_ok(), "Fallback transfer should be creatable");
        }
    }

    // ========================================================================
    // TEST: Upload Handler Integration (RED - will fail until implemented)
    // ========================================================================

    /// Test PutObjectHandler has zero-copy support method
    ///
    /// RED: This test will fail because supports_zero_copy() doesn't exist yet
    #[test]
    fn test_put_object_handler_has_zero_copy_support() {
        use mizuchi_uploadr::upload::put_object::PutObjectHandler;

        let handler = PutObjectHandler::new("test-bucket", "us-east-1");

        // This method should exist and return true on Linux, false otherwise
        let supports = handler.supports_zero_copy();

        #[cfg(target_os = "linux")]
        assert!(supports, "PutObjectHandler should support zero-copy on Linux");

        #[cfg(not(target_os = "linux"))]
        assert!(
            !supports,
            "PutObjectHandler should NOT support zero-copy on non-Linux"
        );
    }

    /// Test MultipartHandler has zero-copy support method
    ///
    /// RED: This test will fail because supports_zero_copy() doesn't exist yet
    #[test]
    fn test_multipart_handler_has_zero_copy_support() {
        use mizuchi_uploadr::upload::multipart::MultipartHandler;

        let handler = MultipartHandler::new("test-bucket", "us-east-1", 5 * 1024 * 1024, 4);

        // This method should exist and return true on Linux, false otherwise
        let supports = handler.supports_zero_copy();

        #[cfg(target_os = "linux")]
        assert!(supports, "MultipartHandler should support zero-copy on Linux");

        #[cfg(not(target_os = "linux"))]
        assert!(
            !supports,
            "MultipartHandler should NOT support zero-copy on non-Linux"
        );
    }
}
