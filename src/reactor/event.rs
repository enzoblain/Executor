//! kqueue event wrapper and operations.
//!
//! Wraps libc kevent structures and provides convenient methods for registering
//! and waiting for I/O and timer events.

use libc::{EV_ADD, EV_DELETE, EV_ENABLE, F_GETFL, F_SETFL, O_NONBLOCK, fcntl, kevent};
use std::ptr;

pub(crate) struct Event(kevent);

impl Event {
    pub(crate) const EMPTY: Self = Self(kevent {
        ident: 0,
        filter: 0,
        flags: 0,
        fflags: 0,
        data: 0,
        udata: ptr::null_mut(),
    });

    pub(crate) fn new(ident: usize, filter: i16, timer_ms: Option<isize>) -> Self {
        let data = timer_ms.unwrap_or(0);

        Self(kevent {
            ident,
            filter,
            flags: EV_ADD | EV_ENABLE,
            fflags: 0,
            data,
            udata: ptr::null_mut(),
        })
    }

    pub(crate) fn get_ident(&self) -> usize {
        self.0.ident
    }

    pub(crate) fn get_filter(&self) -> i16 {
        self.0.filter
    }

    pub(crate) fn register(&self, queue: i32) {
        unsafe {
            kevent(queue, &self.0, 1, ptr::null_mut(), 0, ptr::null());
        }
    }

    pub(crate) fn unregister(queue: i32, ident: usize, filter: i16) {
        let mut event = Self::new(ident, filter, None);
        event.0.flags = EV_DELETE;

        event.register(queue);
    }

    pub(crate) fn wait(queue: i32, events: &mut [Event; 64]) -> i32 {
        unsafe {
            kevent(
                queue,
                ptr::null(),
                0,
                events.as_mut_ptr() as *mut kevent,
                events.len() as i32,
                ptr::null(),
            )
        }
    }

    /// Wait with a timeout (in milliseconds).
    pub(crate) fn wait_with_timeout(queue: i32, events: &mut [Event; 64], timeout_ms: u64) -> i32 {
        let ts = libc::timespec {
            tv_sec: (timeout_ms / 1000) as i64,
            tv_nsec: ((timeout_ms % 1000) * 1_000_000) as i64,
        };

        unsafe {
            kevent(
                queue,
                ptr::null(),
                0,
                events.as_mut_ptr() as *mut kevent,
                events.len() as i32,
                &ts as *const libc::timespec,
            )
        }
    }

    /// Non-blocking wait: polls for any available events and returns immediately.
    pub(crate) fn try_wait(queue: i32, events: &mut [Event; 64]) -> i32 {
        let ts = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };

        unsafe {
            kevent(
                queue,
                ptr::null(),
                0,
                events.as_mut_ptr() as *mut kevent,
                events.len() as i32,
                &ts as *const libc::timespec,
            )
        }
    }

    pub(crate) fn set_nonblocking(file_descriptor: i32) {
        let flags = unsafe { fcntl(file_descriptor, F_GETFL) };

        unsafe {
            fcntl(file_descriptor, F_SETFL, flags | O_NONBLOCK);
        }
    }
}
