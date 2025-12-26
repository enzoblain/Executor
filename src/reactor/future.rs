//! Async read and write primitives.
//!
//! Provides `AsyncRead` and `AsyncWrite` futures that work with the reactor
//! to enable non-blocking I/O operations.

use crate::reactor::core::with_current_reactor;
use std::future::Future;
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

/// A future that completes when a file descriptor is readable.
pub struct AsyncRead {
    file_descriptor: i32,
    registered: bool,
}

impl AsyncRead {
    /// Creates a new AsyncRead for the given file descriptor.
    pub fn new(file_descriptor: i32) -> Self {
        Self {
            file_descriptor,
            registered: false,
        }
    }
}

impl Future for AsyncRead {
    type Output = io::Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.registered {
            let _ = with_current_reactor(|r| {
                r.register_read(self.file_descriptor, cx.waker().clone());
            });
            self.registered = true;

            return Poll::Pending;
        }

        Poll::Ready(Ok(()))
    }
}

/// A future that completes when a file descriptor is writable.
pub struct AsyncWrite {
    file_descriptor: i32,
    registered: bool,
}

impl AsyncWrite {
    /// Creates a new AsyncWrite for the given file descriptor.
    pub fn new(file_descriptor: i32) -> Self {
        Self {
            file_descriptor,
            registered: false,
        }
    }
}

impl Future for AsyncWrite {
    type Output = io::Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if !self.registered {
            let _ = with_current_reactor(|r| {
                r.register_write(self.file_descriptor, cx.waker().clone());
            });
            self.registered = true;

            return Poll::Pending;
        }

        Poll::Ready(Ok(()))
    }
}
