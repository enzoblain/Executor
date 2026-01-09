use crate::reactor::core::ReactorHandle;
use crate::runtime::context::current_reactor_io;

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};
use std::time::Duration;

pub struct Sleep {
    duration: Duration,
    reactor: ReactorHandle,
    registered: bool,
    expired: Arc<AtomicBool>,
}

impl Sleep {
    pub(crate) fn new(duration: Duration) -> Self {
        Self::new_with_reactor(duration, current_reactor_io())
    }

    pub(crate) fn new_with_reactor(duration: Duration, reactor: ReactorHandle) -> Self {
        Self {
            duration,
            reactor,
            registered: false,
            expired: Arc::new(AtomicBool::new(false)),
        }
    }
}

pub fn sleep(duration: Duration) -> Sleep {
    Sleep::new(duration)
}

impl Future for Sleep {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.duration.is_zero() {
            return Poll::Ready(());
        }

        if self.expired.load(Ordering::Acquire) {
            return Poll::Ready(());
        }

        if !self.registered {
            let waker = cx.waker().clone();
            let expired = self.expired.clone();

            self.reactor.lock().unwrap().register_timer_with_callback(
                self.duration,
                waker,
                expired,
            );

            self.registered = true;
        }

        Poll::Pending
    }
}
