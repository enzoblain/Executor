use crate::reactor::event::Event;
use crate::reactor::io::{Connexion, ConnexionState};
use crate::reactor::socket::accept_client;

use libc::{
    EAGAIN, EVFILT_READ, EVFILT_TIMER, EVFILT_WRITE, EWOULDBLOCK, close, kqueue, read, write,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::task::Waker;
use std::time::Duration;

/// A shared handle to a reactor, enabling multiple futures to access the same reactor.
///
/// This allows futures to register I/O and timer events with the reactor.
/// Using `Rc<RefCell<Reactor>>` enables interior mutability and shared ownership
/// while keeping everything single-threaded, preparing for potential multi-threaded
/// designs where this would become `Arc<Mutex<Reactor>>`.
pub type ReactorHandle = Rc<RefCell<Reactor>>;

pub(crate) enum Entry {
    #[allow(unused)]
    Listener,
    Client(Connexion),
    Waiting(Waker),
    // Timer,
}

/// The reactor manages I/O events and timers using kqueue.
///
/// This is typically wrapped in a `ReactorHandle` (Rc<RefCell<Reactor>>)
/// to allow shared access from multiple futures.
pub struct Reactor {
    queue: i32,
    events: [Event; 64],
    n_events: i32,
    registry: HashMap<i32, Entry>,
    timers: HashMap<usize, Waker>,
    next_timer_id: usize,
    wakers: Vec<Waker>,
}

const OUT_MAX_BYTES: usize = 8 * 1024 * 1024;

impl Reactor {
    pub(crate) fn new() -> Self {
        Self {
            queue: unsafe { kqueue() },
            events: [Event::EMPTY; 64],
            n_events: 0,
            registry: HashMap::new(),
            timers: HashMap::new(),
            next_timer_id: 1,
            wakers: Vec::new(),
        }
    }

    pub(crate) fn register_read(&mut self, file_descriptor: i32, waker: Waker) {
        let event = Event::new(file_descriptor as usize, EVFILT_READ, None);
        event.register(self.queue);

        self.registry.insert(file_descriptor, Entry::Waiting(waker));
    }

    pub(crate) fn register_write(&mut self, file_descriptor: i32, waker: Waker) {
        let event = Event::new(file_descriptor as usize, EVFILT_WRITE, None);
        event.register(self.queue);

        self.registry.insert(file_descriptor, Entry::Waiting(waker));
    }

    pub(crate) fn register_timer(&mut self, duration: Duration, waker: Waker) {
        let ms = duration.as_millis().clamp(0, isize::MAX as u128) as isize;
        let id = self.next_timer_id;
        self.next_timer_id = self.next_timer_id.wrapping_add(1).max(1);

        let event = Event::new(id, EVFILT_TIMER, Some(ms));
        event.register(self.queue);

        self.timers.insert(id, waker);
    }

    fn unregister_write(&self, file_descriptor: i32) {
        Event::unregister(self.queue, file_descriptor as usize, EVFILT_WRITE);
    }

    pub(crate) fn wait_for_event(&mut self) {
        let n_events = Event::wait(self.queue, &mut self.events);

        self.n_events = n_events;
    }

    /// Polls for I/O events without blocking and handles them if present.
    pub(crate) fn poll_events(&mut self) {
        let n_events = Event::try_wait(self.queue, &mut self.events);
        if n_events <= 0 {
            return;
        }
        self.n_events = n_events;
        self.handle_events();
    }

    pub(crate) fn wake_ready(&mut self) {
        for waker in self.wakers.drain(..) {
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
                        Entry::Waiting(waker) => {
                            // Preserve the waiting registration so subsequent readiness events
                            // continue to wake the associated future.
                            self.wakers.push(waker.clone());
                            self.registry.insert(file_descriptor, entry);
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
                        Entry::Waiting(waker) => {
                            // Preserve the waiting registration so subsequent readiness events
                            // continue to wake the associated future.
                            self.wakers.push(waker.clone());
                            self.registry.insert(file_descriptor, entry);
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
                    let timer_id = event.get_ident();
                    if let Some(waker) = self.timers.remove(&timer_id) {
                        self.wakers.push(waker);
                    }
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

pub fn errno() -> i32 {
    unsafe { *libc::__error() }
}
