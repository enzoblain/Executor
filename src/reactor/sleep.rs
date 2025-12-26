//! Sleep futures for asynchronous delays.
//!
//! Provides `Sleep` future that completes after a specified duration using reactor timers.

use crate::reactor::core::with_current_reactor;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

/// A future that completes after a specified duration using reactor timers.
#[derive(Debug)]
pub struct Sleep {
    duration: Duration,
    registered: bool,
}

impl Sleep {
    pub fn new(duration: Duration) -> Self {
        Self {
            duration,
            registered: false,
        }
    }
}

impl Future for Sleep {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.duration.is_zero() {
            return Poll::Ready(());
        }

        if !self.registered {
            let w = cx.waker().clone();
            let registered = with_current_reactor(|r| {
                r.register_timer(self.duration, w);
            })
            .is_some();

            if !registered {
                panic!("sleep() called outside of a runtime/reactor context");
            }

            self.registered = true;

            return Poll::Pending;
        }

        Poll::Ready(())
    }
}

/// Creates a sleep future for the given duration.
pub fn sleep(duration: Duration) -> Sleep {
    Sleep::new(duration)
}
