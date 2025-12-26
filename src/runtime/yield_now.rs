//! Cooperative task yielding.
//!
//! Provides a mechanism for tasks to yield control to other tasks.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// Cooperative scheduler hint: yields once to let other tasks run.
///
/// Similar to `tokio::task::yield_now()`, this returns a future that
/// yields `Pending` the first time it's polled and immediately schedules
/// the current task to be polled again by calling `cx.waker().wake_by_ref()`.
pub async fn yield_now() {
    struct YieldOnce(bool);

    impl Future for YieldOnce {
        type Output = ();

        fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
            if !self.0 {
                self.0 = true;
                cx.waker().wake_by_ref();

                return Poll::Pending;
            }

            Poll::Ready(())
        }
    }

    YieldOnce(false).await
}
