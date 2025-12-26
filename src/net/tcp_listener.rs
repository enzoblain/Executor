//! TCP listener for accepting incoming connections.
//!
//! Provides a non-blocking [`TcpListener`] for server applications.
//!
//! # Usage
//!
//! ```ignore
//! use reactor::net::tcp_listener::TcpListener;
//! use std::time::Duration;
//!
//! async fn server() {
//!     let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
//!     println!("Listening on {}", listener.local_addr().unwrap());
//!
//!     loop {
//!         let (stream, peer_addr) = listener.accept().await.unwrap();
//!         println!("New connection from {}", peer_addr);
//!     }
//! }
//! ```
use crate::net::future::AcceptFuture;
use crate::net::tcp_stream::TcpStream;
use crate::net::utils::sockaddr_to_socketaddr;
use crate::reactor::core::ReactorHandle;
use crate::reactor::event::Event;
use crate::runtime::context::current_reactor;

use libc::{AF_INET, SOCK_STREAM, bind, close, getsockname, listen, sockaddr, sockaddr_in, socket};
use std::io;
use std::mem;
use std::net::SocketAddr;

/// A TCP listener that accepts incoming connections.
///
/// `TcpListener` binds to an address and waits for incoming connections. Each
/// call to [`Self::accept`] returns a new [`TcpStream`] and the peer's address.
///
/// # Non-blocking
///
/// All operations are non-blocking. If no connection is available, the listener yields
/// control to other tasks via the reactor.
///
/// # Example
///
/// ```ignore
/// let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
/// loop {
///     let (stream, addr) = listener.accept().await.unwrap();
///     println!("Connection from {}", addr);
/// }
/// ```
pub struct TcpListener {
    file_descriptor: i32,
    reactor: ReactorHandle,
}

impl TcpListener {
    /// Binds a listener to the given address.
    ///
    /// This method performs the following:
    /// 1. Creates a new socket
    /// 2. Sets it to non-blocking mode
    /// 3. Binds to the specified address
    /// 4. Starts listening with a backlog of 128
    ///
    /// # Arguments
    /// * `address` - Address to bind to, format: \"ip:port\" (e.g., \"127.0.0.1:8080\")
    ///
    /// # Returns
    /// A [`TcpListener`] on success, or an I/O error
    ///
    /// # Example
    /// ```ignore
    /// let listener = TcpListener::bind("0.0.0.0:8080").await?;
    /// ```
    pub async fn bind(address: &str) -> io::Result<Self> {
        Self::bind_with_reactor(address, current_reactor()).await
    }

    /// Binds a listener to the given address with a specific reactor handle.
    ///
    /// This is the explicit version of [`bind`](Self::bind) that allows passing
    /// a specific reactor. Most users should use [`bind`](Self::bind) instead,
    /// which automatically uses the current runtime's reactor.
    ///
    /// # Arguments
    /// * `address` - Address to bind to, format: \"ip:port\" (e.g., \"127.0.0.1:8080\")
    /// * `reactor` - A handle to the reactor for managing I/O events
    ///
    /// # Returns
    /// A [`TcpListener`] on success, or an I/O error
    pub async fn bind_with_reactor(address: &str, reactor: ReactorHandle) -> io::Result<Self> {
        let addr = crate::net::utils::parse_sockaddr(address)?;
        let file_descriptor = unsafe { socket(AF_INET, SOCK_STREAM, 0) };

        Event::set_nonblocking(file_descriptor);

        let ret = unsafe {
            bind(
                file_descriptor,
                &addr as *const _ as *const sockaddr,
                mem::size_of::<sockaddr_in>() as u32,
            )
        };

        if ret < 0 {
            return Err(io::Error::last_os_error());
        }

        let ret = unsafe { listen(file_descriptor, 128) };
        if ret < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(Self {
            file_descriptor,
            reactor,
        })
    }

    /// Accepts a new incoming connection.
    ///
    /// Waits for a client to connect to this listener. When a connection is available,
    /// returns a [`TcpStream`] for the new connection and the peer's address.
    ///
    /// # Returns
    /// A tuple of:
    /// - [`TcpStream`]: The accepted connection
    /// - [`SocketAddr`]: The peer's address
    ///
    /// Or an I/O error if the operation fails.
    ///
    /// # Example
    /// ```text
    /// let (stream, addr) = listener.accept().await?;
    /// println!("Accepted connection from {}", addr);
    /// ```
    pub async fn accept(&self) -> io::Result<(TcpStream, SocketAddr)> {
        let (file_descriptor, address) =
            AcceptFuture::new(self.file_descriptor, self.reactor.clone()).await?;

        Ok((
            TcpStream::new(file_descriptor, self.reactor.clone()),
            address,
        ))
    }

    /// Returns the local address this listener is bound to.
    ///
    /// # Returns
    /// The [`SocketAddr`] that this listener is bound to
    ///
    /// # Example
    /// ```text
    /// let listener = TcpListener::bind("127.0.0.1:0").await?;
    /// println!("Listening on {}", listener.local_addr()?);
    /// ```
    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        let mut addr: sockaddr_in = unsafe { mem::zeroed() };
        let mut length = mem::size_of::<sockaddr_in>() as u32;
        let result = unsafe {
            getsockname(
                self.file_descriptor,
                &mut addr as *mut _ as *mut sockaddr,
                &mut length,
            )
        };

        if result < 0 {
            return Err(io::Error::last_os_error());
        }

        Ok(sockaddr_to_socketaddr(&addr))
    }
}

impl Drop for TcpListener {
    fn drop(&mut self) {
        unsafe {
            close(self.file_descriptor);
        }
    }
}
