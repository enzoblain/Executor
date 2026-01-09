use crate::reactor::core::ReactorHandle;
use crate::runtime::context::current_reactor_io;

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

pub fn timeout<F>(duration: Duration, future: F) -> Timeout<F>
where
    F: Future,
{
    Timeout::new(duration, future)
}

pub struct Timeout<F> {
    future: F,
    deadline: Instant,
    duration: Duration,
    reactor: ReactorHandle,
    registered: bool,
}

impl<F> Timeout<F> {
    pub(crate) fn new(duration: Duration, future: F) -> Self {
        Timeout {
            future,
            deadline: Instant::now() + duration,
            duration,
            reactor: current_reactor_io(),
            registered: false,
        }
    }
}

impl<F: Future> Future for Timeout<F> {
    type Output = Result<F::Output, ()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if Instant::now() >= self.deadline {
            return Poll::Ready(Err(()));
        }

        let future = unsafe { self.as_mut().map_unchecked_mut(|s| &mut s.future) };
        if let Poll::Ready(v) = future.poll(cx) {
            return Poll::Ready(Ok(v));
        }

        if !self.registered {
            let waker = cx.waker().clone();

            self.reactor
                .lock()
                .unwrap()
                .register_timer(self.duration, waker);

            unsafe {
                let this = self.get_unchecked_mut();
                this.registered = true;
            }
        }

        Poll::Pending
    }
}
