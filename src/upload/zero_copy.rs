//! Zero-copy transfer implementation
//!
//! Uses Linux splice(2)/sendfile(2) for kernel-space transfers.
//! Falls back to tokio buffered I/O on other platforms.

use super::UploadError;
use std::io;

/// Default buffer size for transfers
pub const DEFAULT_BUFFER_SIZE: usize = 65536; // 64KB

// ============================================================================
// Linux Implementation (Zero-Copy)
// ============================================================================

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use nix::fcntl::{splice, SpliceFFlags};
    use nix::unistd::pipe;
    use std::os::fd::IntoRawFd;
    use std::os::unix::io::{AsRawFd, RawFd};

    /// Zero-copy transfer using Linux splice(2)
    pub struct ZeroCopyTransfer {
        pipe_read: RawFd,
        pipe_write: RawFd,
        buffer_size: usize,
    }

    impl ZeroCopyTransfer {
        /// Create a new zero-copy transfer
        pub fn new(buffer_size: usize) -> io::Result<Self> {
            let (pipe_read, pipe_write) = pipe().map_err(|e| {
                io::Error::new(io::ErrorKind::Other, format!("pipe() failed: {}", e))
            })?;

            Ok(Self {
                pipe_read: pipe_read.into_raw_fd(),
                pipe_write: pipe_write.into_raw_fd(),
                buffer_size,
            })
        }

        /// Transfer data from source to destination using splice
        pub async fn transfer<S, D>(&self, source: &S, dest: &D, len: usize) -> io::Result<usize>
        where
            S: AsRawFd,
            D: AsRawFd,
        {
            let source_fd = source.as_raw_fd();
            let dest_fd = dest.as_raw_fd();
            let mut total_transferred = 0;
            let mut remaining = len;

            while remaining > 0 {
                let chunk_size = std::cmp::min(remaining, self.buffer_size);

                // Splice from source to pipe
                let spliced_to_pipe = match splice(
                    source_fd,
                    None,
                    self.pipe_write,
                    None,
                    chunk_size,
                    SpliceFFlags::SPLICE_F_MOVE | SpliceFFlags::SPLICE_F_NONBLOCK,
                ) {
                    Ok(n) if n == 0 => break, // EOF
                    Ok(n) => n,
                    Err(nix::errno::Errno::EAGAIN) => {
                        tokio::task::yield_now().await;
                        continue;
                    }
                    Err(e) => {
                        return Err(io::Error::new(
                            io::ErrorKind::Other,
                            format!("splice to pipe failed: {}", e),
                        ));
                    }
                };

                // Splice from pipe to destination
                let mut pipe_remaining = spliced_to_pipe;
                while pipe_remaining > 0 {
                    match splice(
                        self.pipe_read,
                        None,
                        dest_fd,
                        None,
                        pipe_remaining,
                        SpliceFFlags::SPLICE_F_MOVE | SpliceFFlags::SPLICE_F_NONBLOCK,
                    ) {
                        Ok(n) => {
                            pipe_remaining -= n;
                            total_transferred += n;
                            remaining -= n;
                        }
                        Err(nix::errno::Errno::EAGAIN) => {
                            tokio::task::yield_now().await;
                        }
                        Err(e) => {
                            return Err(io::Error::new(
                                io::ErrorKind::Other,
                                format!("splice from pipe failed: {}", e),
                            ));
                        }
                    }
                }
            }

            Ok(total_transferred)
        }
    }

    impl Drop for ZeroCopyTransfer {
        fn drop(&mut self) {
            unsafe {
                libc::close(self.pipe_read);
                libc::close(self.pipe_write);
            }
        }
    }

    /// Check if zero-copy is available
    pub fn is_available() -> bool {
        true
    }
}

// ============================================================================
// Fallback Implementation (Buffered I/O)
// ============================================================================

#[cfg(not(target_os = "linux"))]
mod fallback {
    use super::*;
    use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

    /// Buffered transfer (fallback for non-Linux platforms)
    pub struct ZeroCopyTransfer {
        buffer_size: usize,
    }

    impl ZeroCopyTransfer {
        /// Create a new buffered transfer
        pub fn new(buffer_size: usize) -> io::Result<Self> {
            Ok(Self { buffer_size })
        }

        /// Transfer data using buffered I/O
        pub async fn transfer<S, D>(
            &self,
            source: &mut S,
            dest: &mut D,
            len: usize,
        ) -> io::Result<usize>
        where
            S: AsyncRead + Unpin,
            D: AsyncWrite + Unpin,
        {
            let mut buffer = vec![0u8; self.buffer_size];
            let mut total_transferred = 0;
            let mut remaining = len;

            while remaining > 0 {
                let to_read = std::cmp::min(remaining, self.buffer_size);
                let n = source.read(&mut buffer[..to_read]).await?;
                if n == 0 {
                    break; // EOF
                }

                dest.write_all(&buffer[..n]).await?;
                total_transferred += n;
                remaining -= n;
            }

            dest.flush().await?;
            Ok(total_transferred)
        }
    }

    /// Check if zero-copy is available
    pub fn is_available() -> bool {
        false
    }
}

// ============================================================================
// Platform-Agnostic API
// ============================================================================

#[cfg(target_os = "linux")]
pub use linux::{is_available, ZeroCopyTransfer};

#[cfg(not(target_os = "linux"))]
pub use fallback::{is_available, ZeroCopyTransfer};

/// Data transfer abstraction
pub struct DataTransfer {
    #[allow(dead_code)] // Will be used when transfer methods are implemented
    inner: ZeroCopyTransfer,
    use_zero_copy: bool,
}

impl DataTransfer {
    /// Create a new data transfer
    pub fn new(buffer_size: usize, use_zero_copy: bool) -> Result<Self, UploadError> {
        let inner = ZeroCopyTransfer::new(buffer_size)?;
        Ok(Self {
            inner,
            use_zero_copy: use_zero_copy && is_available(),
        })
    }

    /// Check if zero-copy is being used
    pub fn is_zero_copy(&self) -> bool {
        self.use_zero_copy
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_available() {
        let available = is_available();
        #[cfg(target_os = "linux")]
        assert!(available);
        #[cfg(not(target_os = "linux"))]
        assert!(!available);
    }

    #[test]
    fn test_data_transfer_creation() {
        let transfer = DataTransfer::new(DEFAULT_BUFFER_SIZE, true);
        assert!(transfer.is_ok());
    }
}
