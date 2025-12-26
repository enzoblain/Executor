//! TCP stream for non-blocking read and write operations.
//!
//! Provides a non-blocking [`TcpStream`] for client connections.
//!
//! # Usage
//!
//! ```ignore
//! use reactor::net::tcp_stream::TcpStream;
//!
//! async fn read_and_echo(stream: TcpStream) {
//!     let mut buf = [0u8; 1024];
//!     loop {
//!         match stream.read(&mut buf).await {
//!             Ok(0) => break, // Connection closed
//!             Ok(n) => {
//!                 stream.write_all(&buf[..n]).await.ok();
//!             }
//!             Err(e) => eprintln!("Error: {}", e),
//!         }
//!     }
//! }
//! ```
use crate::net::future::{ReadFuture, WriteFuture};
use crate::reactor::core::ReactorHandle;

use libc::close;
use std::io;

/// A TCP stream for non-blocking I/O operations.
///
/// `TcpStream` wraps a socket file descriptor and provides async read/write methods.
/// All operations are non-blocking and will yield control to other tasks when data
/// is not immediately available.
///
/// # Methods
///
/// - [`Self::read`]: Read available data (may return 0 or partial data)
/// - [`Self::write`]: Write data (may write less than requested)
/// - [`Self::write_all`]: Write all data, retrying until complete
///
/// # Example
///
/// ```ignore
/// let mut buf = [0u8; 1024];
/// match stream.read(&mut buf).await {
///     Ok(n) if n > 0 => println!("Read {} bytes", n),
///     Ok(_) => println!("Connection closed"),
///     Err(e) => eprintln!("Error: {}", e),
/// }
/// ```
pub struct TcpStream {
    file_descriptor: i32,
    reactor: ReactorHandle,
}

impl TcpStream {
    /// Creates a new TCP stream from a file descriptor.
    ///
    /// # Arguments
    /// * `file_descriptor` - An open socket file descriptor (must be non-blocking)
    /// * `reactor` - A handle to the reactor for managing I/O events
    ///
    /// # Safety
    ///
    /// The caller must ensure the file descriptor is valid and represents an open socket.
    pub fn new(file_descriptor: i32, reactor: ReactorHandle) -> Self {
        Self {
            file_descriptor,
            reactor,
        }
    }

    /// Reads data from the stream.
    ///
    /// Attempts to read available data into the provided buffer. Returns the number
    /// of bytes read, or 0 if the connection is closed.
    ///
    /// # Arguments
    /// * `buffer` - A mutable buffer to read data into
    ///
    /// # Returns
    /// - `Ok(n)` where n > 0: Data was read
    /// - `Ok(0)`: Connection closed
    /// - `Err(e)`: I/O error
    ///
    /// # Note
    ///
    /// This may return fewer bytes than the buffer size. See [`Self::write_all`]
    /// for a method that retries until all data is written.
    pub fn read<'a>(&'a self, buffer: &'a mut [u8]) -> ReadFuture<'a> {
        ReadFuture::new(self.file_descriptor, buffer, self.reactor.clone())
    }

    /// Writes data to the stream.
    ///
    /// Attempts to write data to the stream. Returns the number of bytes written.
    ///
    /// # Arguments
    /// * `buffer` - A buffer containing data to write
    ///
    /// # Returns
    /// - `Ok(n)` where n > 0: Data was written
    /// - `Ok(0)`: This shouldn't happen with non-blocking sockets
    /// - `Err(e)`: I/O error
    ///
    /// # Note
    ///
    /// This may write fewer bytes than the buffer size. Use [`Self::write_all`]
    /// to ensure all data is written.
    pub fn write<'a>(&'a self, buffer: &'a [u8]) -> WriteFuture<'a> {
        WriteFuture::new(self.file_descriptor, buffer, self.reactor.clone())
    }

    /// Writes all data to the stream, retrying until complete.
    ///
    /// This method keeps writing until all data is written or an error occurs.
    ///
    /// # Arguments
    /// * `buffer` - Data to write
    ///
    /// # Returns
    /// - `Ok(())` if all data was written
    /// - `Err(e)` if an error occurs, including `WriteZero` if write returns 0
    ///
    /// # Example
    ///
    /// ```text
    /// stream.write_all(b"Hello, World!").await?;
    /// ```
    pub async fn write_all(&self, mut buffer: &[u8]) -> io::Result<()> {
        while !buffer.is_empty() {
            let n = self.write(buffer).await?;
            if n == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::WriteZero,
                    "write returned zero bytes",
                ));
            }
            buffer = &buffer[n..];
        }

        Ok(())
    }
}

impl Drop for TcpStream {
    fn drop(&mut self) {
        unsafe {
            close(self.file_descriptor);
        }
    }
}
