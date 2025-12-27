//! Temporary file upload helper for zero-copy transfers
//!
//! This module provides a temp-file-based approach for large uploads,
//! enabling zero-copy transfers via `sendfile(2)` on Linux.
//!
//! # Flow
//!
//! 1. Write incoming body to temp file (streaming, minimal memory)
//! 2. Compute SHA256 hash for SigV4 signing
//! 3. Use sendfile for zero-copy transfer to S3
//!
//! # Example
//!
//! ```no_run
//! use mizuchi_uploadr::upload::temp_file::TempFileUpload;
//! use bytes::Bytes;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let data = Bytes::from("Hello, World!");
//! let temp = TempFileUpload::from_bytes(data)?;
//!
//! println!("File: {:?}", temp.path());
//! println!("Size: {} bytes", temp.size());
//! println!("SHA256: {}", temp.content_hash());
//! # Ok(())
//! # }
//! ```

use bytes::Bytes;
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

#[cfg(target_os = "linux")]
use std::os::fd::{AsFd, BorrowedFd};

use super::UploadError;

/// Temporary file for zero-copy uploads
///
/// Automatically cleaned up when dropped (RAII pattern).
pub struct TempFileUpload {
    path: PathBuf,
    file: File,
    size: u64,
    content_hash: String,
}

impl TempFileUpload {
    /// Create a temp file from Bytes
    ///
    /// Writes the data to a temp file and computes SHA256 hash.
    /// Uses tmpfs (/dev/shm) on Linux when available for better performance.
    pub fn from_bytes(data: Bytes) -> Result<Self, UploadError> {
        // Choose temp directory: prefer tmpfs on Linux
        let temp_dir = Self::get_temp_dir();

        // Create temp file
        let file_name = format!("mizuchi-{}.tmp", uuid::Uuid::new_v4());
        let path = temp_dir.join(file_name);

        // Write data to file
        let mut file = File::create(&path)?;
        file.write_all(&data)?;
        file.flush()?;

        // Compute SHA256 hash
        let content_hash = Self::compute_sha256(&data);

        // Reopen for reading
        let file = File::open(&path)?;

        Ok(Self {
            path,
            file,
            size: data.len() as u64,
            content_hash,
        })
    }

    /// Get the path to the temp file
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the size of the file in bytes
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Get the SHA256 hash of the content (hex encoded)
    ///
    /// This is used for the `x-amz-content-sha256` header in SigV4 signing.
    pub fn content_hash(&self) -> &str {
        &self.content_hash
    }

    /// Check if zero-copy (sendfile) is available on this platform
    pub fn supports_zero_copy(&self) -> bool {
        cfg!(target_os = "linux")
    }

    /// Get a reference to the underlying file
    pub fn file(&self) -> &File {
        &self.file
    }

    /// Get a mutable reference to the underlying file
    pub fn file_mut(&mut self) -> &mut File {
        &mut self.file
    }

    /// Read all content into a buffer
    ///
    /// This is a fallback for platforms without zero-copy support.
    pub fn read_all(&mut self) -> io::Result<Vec<u8>> {
        let mut buffer = Vec::with_capacity(self.size as usize);
        self.file.read_to_end(&mut buffer)?;
        Ok(buffer)
    }

    // ========================================================================
    // Private helpers
    // ========================================================================

    /// Get the best temp directory for the platform
    fn get_temp_dir() -> PathBuf {
        #[cfg(target_os = "linux")]
        {
            // Prefer /dev/shm (tmpfs) on Linux
            let shm = PathBuf::from("/dev/shm");
            if shm.exists() && shm.is_dir() {
                return shm;
            }
        }

        // Fallback to system temp dir
        std::env::temp_dir()
    }

    /// Compute SHA256 hash of data
    fn compute_sha256(data: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(data);
        hex::encode(hasher.finalize())
    }
}

// Linux-specific: file descriptor access for sendfile
#[cfg(target_os = "linux")]
impl AsFd for TempFileUpload {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.file.as_fd()
    }
}

impl Drop for TempFileUpload {
    fn drop(&mut self) {
        // Clean up temp file
        if self.path.exists() {
            if let Err(e) = std::fs::remove_file(&self.path) {
                tracing::warn!(
                    path = %self.path.display(),
                    error = %e,
                    "Failed to clean up temp file"
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_temp_file() {
        let data = Bytes::from("test data");
        let temp = TempFileUpload::from_bytes(data).unwrap();

        assert!(temp.path().exists());
        assert_eq!(temp.size(), 9);
    }

    #[test]
    fn test_content_hash() {
        let data = Bytes::from("hello");
        let temp = TempFileUpload::from_bytes(data).unwrap();

        assert_eq!(
            temp.content_hash(),
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn test_cleanup_on_drop() {
        let path;
        {
            let data = Bytes::from("temp data");
            let temp = TempFileUpload::from_bytes(data).unwrap();
            path = temp.path().to_path_buf();
            assert!(path.exists());
        }
        // Dropped
        assert!(!path.exists());
    }
}
