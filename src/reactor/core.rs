use crate::reactor::event::Event;
use crate::reactor::io::{Connexion, ConnexionState};
use crate::reactor::socket::accept_client;

use libc::{
    EAGAIN, EVFILT_READ, EVFILT_TIMER, EVFILT_WRITE, EWOULDBLOCK, close, kqueue, read, write,
};
use std::collections::HashMap;
use std::sync::Arc;
use std::task::Waker;

pub(crate) enum Entry {
    #[allow(unused)]
    Listener,
    Client(Connexion),
    /// A future waiting for I/O events
    Future(FutureEntry),
    // Timer,
}

/// Represents a future waiting for I/O readiness
pub(crate) struct FutureEntry {
    /// Waker to notify when the I/O event is ready
    pub(crate) waker: Waker,
    /// Type of I/O event being waited for
    pub(crate) interest: Interest,
}

/// Type of I/O event interest
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Interest {
    Read,
    Write,
    ReadWrite,
}

pub(crate) struct Reactor {
    queue: i32,
    events: [Event; 64],
    n_events: i32,
    registry: HashMap<i32, Entry>,
    /// Pending wakers to be notified
    pending_wakers: Vec<Waker>,
}

const OUT_MAX_BYTES: usize = 8 * 1024 * 1024;

impl Reactor {
    pub(crate) fn new() -> Self {
        Self {
            queue: unsafe { kqueue() },
            events: [Event::EMPTY; 64],
            n_events: 0,
            registry: HashMap::new(),
            pending_wakers: Vec::new(),
        }
    }

    /// Register a future to be woken when I/O is ready
    pub(crate) fn register_future(
        &mut self,
        file_descriptor: i32,
        waker: Waker,
        interest: Interest,
    ) {
        // Register appropriate kqueue events based on interest
        match interest {
            Interest::Read => {
                let event = Event::new(file_descriptor as usize, EVFILT_READ, None);
                event.register(self.queue);
            }
            Interest::Write => {
                let event = Event::new(file_descriptor as usize, EVFILT_WRITE, None);
                event.register(self.queue);
            }
            Interest::ReadWrite => {
                let event_read = Event::new(file_descriptor as usize, EVFILT_READ, None);
                let event_write = Event::new(file_descriptor as usize, EVFILT_WRITE, None);
                event_read.register(self.queue);
                event_write.register(self.queue);
            }
        }

        // Store the future entry
        self.registry.insert(
            file_descriptor,
            Entry::Future(FutureEntry { waker, interest }),
        );
    }

    /// Unregister a future from I/O notifications
    pub(crate) fn unregister_future(&mut self, file_descriptor: i32, interest: Interest) {
        match interest {
            Interest::Read => {
                Event::unregister(self.queue, file_descriptor as usize, EVFILT_READ);
            }
            Interest::Write => {
                Event::unregister(self.queue, file_descriptor as usize, EVFILT_WRITE);
            }
            Interest::ReadWrite => {
                Event::unregister(self.queue, file_descriptor as usize, EVFILT_READ);
                Event::unregister(self.queue, file_descriptor as usize, EVFILT_WRITE);
            }
        }

        self.registry.remove(&file_descriptor);
    }

    // fn register_event(&self, file_descriptor: usize, filter: i16) {
    //     let event = Event::new(file_descriptor, filter, None);
    //     event.register(self.queue);
    // }

    // fn register_read(&self, file_descriptor: i32) {
    //     self.register_event(file_descriptor as usize, EVFILT_READ);
    // }

    // fn register_write(&self, file_descriptor: i32) {
    //     self.register_event(file_descriptor as usize, EVFILT_WRITE);
    // }

    fn unregister_write(&self, file_descriptor: i32) {
        Event::unregister(self.queue, file_descriptor as usize, EVFILT_WRITE);
    }

    pub(crate) fn wait_for_event(&mut self) {
        let n_events = Event::wait(self.queue, &mut self.events);

        self.n_events = n_events;
    }

    /// Wake all pending futures and clear the list
    pub(crate) fn wake_pending(&mut self) {
        for waker in self.pending_wakers.drain(..) {
            waker.wake();
        }
    }

    pub(crate) fn handle_events(&mut self) {
        for event in self.events.iter().take(self.n_events as usize) {
            let file_descriptor = event.get_ident() as i32;
            let filter = event.get_filter();

            match filter {
                EVFILT_READ
                    if matches!(self.registry.get(&(file_descriptor)), Some(Entry::Listener)) =>
                {
                    accept_client(self.queue, &mut self.registry, file_descriptor);
                }
                EVFILT_READ => {
                    let mut entry = match self.registry.remove(&file_descriptor) {
                        Some(entry) => entry,
                        None => continue,
                    };

                    match &mut entry {
                        Entry::Future(future_entry) => {
                            // Wake the future waiting for read readiness
                            self.pending_wakers.push(future_entry.waker.clone());
                            // Don't re-insert, the future will re-register if needed
                            continue;
                        }
                        Entry::Client(conn) if matches!(conn.state, ConnexionState::Reading) => {
                            let close = self.handle_read(file_descriptor, conn);
                            if close {
                                self.cleanup(file_descriptor);
                            } else {
                                self.registry.insert(file_descriptor, entry);
                            }
                        }
                        Entry::Client(_) => {
                            self.registry.insert(file_descriptor, entry);
                        }
                        _ => {
                            self.cleanup(file_descriptor);
                        }
                    }
                }
                EVFILT_WRITE => {
                    let mut entry = match self.registry.remove(&file_descriptor) {
                        Some(entry) => entry,
                        None => continue,
                    };

                    match &mut entry {
                        Entry::Future(future_entry) => {
                            // Wake the future waiting for write readiness
                            self.pending_wakers.push(future_entry.waker.clone());
                            // Don't re-insert, the future will re-register if needed
                            continue;
                        }
                        Entry::Client(conn) if matches!(conn.state, ConnexionState::Writing) => {
                            let close = self.handle_write(file_descriptor, conn);
                            if close {
                                self.cleanup(file_descriptor);
                            } else {
                                self.registry.insert(file_descriptor, entry);
                            }
                        }
                        Entry::Client(_) => {
                            self.registry.insert(file_descriptor, entry);
                        }
                        _ => {
                            self.cleanup(file_descriptor);
                        }
                    }
                }
                EVFILT_TIMER => {
                    todo!()
                }
                _ => {}
            }
        }
    }

    fn handle_read(&self, file_descriptor: i32, connexion: &mut Connexion) -> bool {
        let mut buf = [0u8; 1024];
        let res = unsafe { read(file_descriptor, buf.as_mut_ptr() as *mut _, buf.len()) };

        if res == 0 {
            return true;
        }

        if res < 0 {
            let err = errno();
            if err == EAGAIN || err == EWOULDBLOCK {
                return false;
            }

            return true;
        }

        let add_len = res as usize;
        if connexion.out.len().saturating_add(add_len) > OUT_MAX_BYTES {
            return true;
        }

        connexion.out.extend_from_slice(&buf[..add_len]);
        connexion.state = ConnexionState::Writing;

        false
    }

    fn handle_write(&self, file_descriptor: i32, connexion: &mut Connexion) -> bool {
        let res = unsafe {
            write(
                file_descriptor,
                connexion.out.as_mut_ptr() as *mut _,
                connexion.out.len(),
            )
        };

        if res < 0 {
            let err = errno();
            if err == EAGAIN || err == EWOULDBLOCK {
                return false;
            }

            return true;
        }

        connexion.out.drain(..res as usize);

        if connexion.out.is_empty() {
            self.unregister_write(file_descriptor);
            connexion.state = ConnexionState::Reading;
        }

        false
    }

    fn cleanup(&self, file_descriptor: i32) {
        Event::unregister(self.queue, file_descriptor as usize, EVFILT_READ);
        Event::unregister(self.queue, file_descriptor as usize, EVFILT_WRITE);
        unsafe { close(file_descriptor) };
    }
}

fn errno() -> i32 {
    unsafe { *libc::__error() }
}
