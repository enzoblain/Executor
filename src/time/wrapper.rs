use crate::JoinHandle;

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

pub struct Time<T: Send + Sync + 'static> {
    start: Instant,
    handle: JoinHandle<T>,
}

impl<T: Send + Sync> Time<T> {
    pub fn new(handle: JoinHandle<T>) -> Self {
        Self {
            start: Instant::now(),
            handle,
        }
    }
}

impl<T: Send + Sync> Future for Time<T> {
    type Output = (T, Duration);

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        match Pin::new(&mut this.handle).poll(cx) {
            Poll::Ready(result) => {
                let elapsed = this.start.elapsed();

                Poll::Ready((result, elapsed))
            }

            Poll::Pending => Poll::Pending,
        }
    }
}
