//! Task wrapper that combines futures with waker integration.
//!
//! A task encapsulates a future and provides mechanisms for polling and awakening
//! when the future is ready to make progress.

use crate::context::enter_context;
use crate::queue::TaskQueue;
use crate::waker::make_waker;

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::Context;

/// A spawned task that wraps a future.
///
/// Contains a boxed future and references to the task queue for re-scheduling
/// when the task is awakened.
pub struct Task {
    future: Mutex<Option<Pin<Box<dyn Future<Output = ()> + Send>>>>,
    pub(crate) queue: Arc<TaskQueue>,
}

impl Task {
    /// Creates a new task wrapping the given future.
    ///
    /// Constructs a new Task by boxing the future and storing it with a reference
    /// to the task queue for re-scheduling upon awakening.
    ///
    /// # Arguments
    /// * `fut` - The future to wrap as a task
    /// * `queue` - The task queue for scheduling this task
    ///
    /// # Returns
    /// An Arc-wrapped Task ready for spawning
    pub(crate) fn new(
        fut: impl Future<Output = ()> + Send + 'static,
        queue: Arc<TaskQueue>,
    ) -> Arc<Self> {
        Arc::new(Task {
            future: Mutex::new(Some(Box::pin(fut))),
            queue,
        })
    }

    /// Polls the task's future once.
    ///
    /// Attempts to make progress on the wrapped future. If the future returns Pending,
    /// it is stored back for later polling. If it returns Ready, the task is complete.
    /// Uses a custom waker to enable task re-scheduling.
    ///
    /// This method also establishes the runtime context, allowing spawned tasks to use
    /// the global `spawn()` function.
    pub fn poll(self: &Arc<Self>) {
        enter_context(self.queue.clone(), || {
            let w = make_waker(self.clone());
            let mut cx = Context::from_waker(&w);

            let mut slot = self.future.lock().unwrap();

            if let Some(mut fut) = slot.take()
                && fut.as_mut().poll(&mut cx).is_pending()
            {
                *slot = Some(fut);
            }
        });
    }

    /// Spawns a subtask from within this task.
    ///
    /// Creates and enqueues a new task to be executed concurrently.
    ///
    /// # Arguments
    /// * `fut` - The future to spawn as a subtask
    pub fn spawn<F: Future<Output = ()> + Send + 'static>(&self, fut: F) {
        let task = Task::new(fut, self.queue.clone());
        self.queue.push(task);
    }
}
