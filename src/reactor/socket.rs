//! Socket acceptance utilities for kqueue.

use crate::reactor::core::Entry;
use crate::reactor::event::Event;
use crate::reactor::io::Connexion;

use libc::{EAGAIN, EMFILE, ENFILE, EVFILT_READ, EWOULDBLOCK, accept};
use std::collections::HashMap;
use std::ptr;

pub(crate) fn accept_client(
    queue: i32,
    registry: &mut HashMap<i32, Entry>,
    listener_file_descriptor: i32,
) {
    let client_file_descriptor =
        unsafe { accept(listener_file_descriptor, ptr::null_mut(), ptr::null_mut()) };

    if client_file_descriptor < 0 {
        let error = errno();
        if error == EAGAIN || error == EWOULDBLOCK {
            return;
        }

        if error == EMFILE || error == ENFILE {
            return;
        }

        return;
    }

    Event::set_nonblocking(client_file_descriptor);

    let event = Event::new(client_file_descriptor as usize, EVFILT_READ, None);
    event.register(queue);

    registry.insert(client_file_descriptor, Entry::Client(Connexion::new()));
}

fn errno() -> i32 {
    unsafe { *libc::__error() }
}
