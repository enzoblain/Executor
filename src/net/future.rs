//! Async TCP futures for accepting connections and reading/writing data.

use crate::net::utils::sockaddr_to_socketaddr;
use crate::reactor::core::ReactorHandle;
use crate::reactor::event::Event;

use libc::{EAGAIN, EWOULDBLOCK, accept, read, sockaddr, sockaddr_in, socklen_t, write};
use std::future::Future;
use std::io;
use std::mem;
use std::net::SocketAddr;
use std::pin::Pin;
use std::task::{Context, Poll};

/// A future that accepts a new client connection.
pub struct AcceptFuture {
    listen_file_descriptor: i32,
    reactor: ReactorHandle,
    registered: bool,
}

impl AcceptFuture {
    pub fn new(listen_file_descriptor: i32, reactor: ReactorHandle) -> Self {
        Self {
            listen_file_descriptor,
            reactor,
            registered: false,
        }
    }
}

impl Future for AcceptFuture {
    type Output = io::Result<(i32, SocketAddr)>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut addr: sockaddr_in = unsafe { mem::zeroed() };
        let mut addr_len: socklen_t = mem::size_of::<sockaddr_in>() as socklen_t;

        let client_fd = unsafe {
            accept(
                self.listen_file_descriptor,
                &mut addr as *mut _ as *mut sockaddr,
                &mut addr_len,
            )
        };

        if client_fd >= 0 {
            Event::set_nonblocking(client_fd);
            let socket_addr = sockaddr_to_socketaddr(&addr);

            return Poll::Ready(Ok((client_fd, socket_addr)));
        }

        let error = unsafe { *libc::__error() };

        if error == EAGAIN || error == EWOULDBLOCK {
            if !self.registered {
                self.reactor
                    .borrow_mut()
                    .register_read(self.listen_file_descriptor, cx.waker().clone());
                self.registered = true;
            }

            return Poll::Pending;
        }

        Poll::Ready(Err(io::Error::last_os_error()))
    }
}

/// A future that reads data from a file descriptor.
pub struct ReadFuture<'a> {
    file_descriptor: i32,
    buffer: &'a mut [u8],
    reactor: ReactorHandle,
    registered: bool,
}

impl<'a> ReadFuture<'a> {
    pub fn new(file_descriptor: i32, buffer: &'a mut [u8], reactor: ReactorHandle) -> Self {
        Self {
            file_descriptor,
            buffer,
            reactor,
            registered: false,
        }
    }
}

impl<'a> Future for ReadFuture<'a> {
    type Output = io::Result<usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.as_mut().get_unchecked_mut() };

        let res = unsafe {
            read(
                this.file_descriptor,
                this.buffer.as_mut_ptr() as *mut _,
                this.buffer.len(),
            )
        };

        if res > 0 {
            return Poll::Ready(Ok(res as usize));
        }

        if res == 0 {
            return Poll::Ready(Ok(0));
        }

        let error = unsafe { *libc::__error() };

        if error == EAGAIN || error == EWOULDBLOCK {
            if !this.registered {
                this.reactor
                    .borrow_mut()
                    .register_read(this.file_descriptor, cx.waker().clone());
                this.registered = true;
            }

            return Poll::Pending;
        }

        Poll::Ready(Err(io::Error::last_os_error()))
    }
}

/// A future that writes data to a file descriptor.
pub struct WriteFuture<'a> {
    file_descriptor: i32,
    buffer: &'a [u8],
    reactor: ReactorHandle,
    registered: bool,
}

impl<'a> WriteFuture<'a> {
    pub fn new(file_descriptor: i32, buffer: &'a [u8], reactor: ReactorHandle) -> Self {
        Self {
            file_descriptor,
            buffer,
            reactor,
            registered: false,
        }
    }
}

impl<'a> Future for WriteFuture<'a> {
    type Output = io::Result<usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.as_mut().get_unchecked_mut() };

        let res = unsafe {
            write(
                this.file_descriptor,
                this.buffer.as_ptr() as *const _,
                this.buffer.len(),
            )
        };

        if res >= 0 {
            return Poll::Ready(Ok(res as usize));
        }

        let error = unsafe { *libc::__error() };

        if error == EAGAIN || error == EWOULDBLOCK {
            if !this.registered {
                this.reactor
                    .borrow_mut()
                    .register_write(this.file_descriptor, cx.waker().clone());
                this.registered = true;
            }

            return Poll::Pending;
        }

        Poll::Ready(Err(io::Error::last_os_error()))
    }
}
