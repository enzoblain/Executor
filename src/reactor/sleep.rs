//! Sleep futures for asynchronous delays.
//!
//! Provides `Sleep` future that completes after a specified duration using reactor timers.

use crate::reactor::core::ReactorHandle;
use crate::runtime::context::current_reactor;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

/// A future that completes after a specified duration using reactor timers.
pub struct Sleep {
    duration: Duration,
    reactor: ReactorHandle,
    registered: bool,
}

impl Sleep {
    /// Creates a new sleep future with the current reactor.
    ///
    /// # Arguments
    /// * `duration` - How long to sleep
    ///
    /// # Example
    /// ```ignore
    /// Sleep::new(Duration::from_secs(1)).await;
    /// ```
    pub fn new(duration: Duration) -> Self {
        Self::new_with_reactor(duration, current_reactor())
    }

    /// Creates a new sleep future with a specific reactor handle.
    ///
    /// This is the explicit version that allows passing a specific reactor.
    /// Most users should use [`new`](Self::new) instead.
    ///
    /// # Arguments
    /// * `duration` - How long to sleep
    /// * `reactor` - A handle to the reactor for managing timer events
    pub fn new_with_reactor(duration: Duration, reactor: ReactorHandle) -> Self {
        Self {
            duration,
            reactor,
            registered: false,
        }
    }
}

/// Creates a sleep future for the given duration.
///
/// # Arguments
/// * `duration` - How long to sleep
///
/// # Example
/// ```ignore
/// sleep(Duration::from_secs(1)).await;
/// ```
pub fn sleep(duration: Duration) -> Sleep {
    Sleep::new(duration)
}

impl Future for Sleep {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.duration.is_zero() {
            return Poll::Ready(());
        }

        if !self.registered {
            let w = cx.waker().clone();
            self.reactor.borrow_mut().register_timer(self.duration, w);
            self.registered = true;

            return Poll::Pending;
        }

        Poll::Ready(())
    }
}
