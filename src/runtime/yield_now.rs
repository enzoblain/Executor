use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

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

pub async fn yield_now() {
    YieldOnce(false).await
}
