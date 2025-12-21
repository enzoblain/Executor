//! Async runtime that executes futures and manages task scheduling.
//!
//! The runtime coordinates the execution of a main future via `block_on` and handles
//! spawned background tasks. It uses a task queue and executor to manage concurrent execution.

use crate::context::enter_context;
use crate::executor::Executor;
use crate::queue::TaskQueue;
use crate::task::Task;
use crate::waker::noop_waker;

use std::future::Future;
use std::sync::Arc;
use std::task::{Context, Poll};

/// Main async runtime for executing futures.
///
/// Provides the core API for running futures to completion and spawning background tasks.
/// The runtime maintains an executor and task queue for managing concurrent task execution.
pub struct Runtime {
    queue: Arc<TaskQueue>,
    executor: Executor,
}

impl Runtime {
    /// Creates a new runtime with an empty task queue and executor.
    ///
    /// Initializes the runtime components needed for executing futures and managing
    /// spawned tasks.
    ///
    /// # Example
    /// ```ignore
    /// let rt = Runtime::new();
    /// ```
    pub fn new() -> Self {
        let queue = Arc::new(TaskQueue::new());
        let executor = Executor::new(queue.clone());
        Self { queue, executor }
    }

    /// Spawns a background task to be executed concurrently.
    ///
    /// Creates a new task from the provided future and enqueues it for execution.
    /// The task will be executed when `block_on` drains the task queue.
    ///
    /// # Arguments
    /// * `fut` - A future with `Output = ()` that will be executed as a background task
    ///
    /// # Example
    /// ```ignore
    /// rt.spawn(async {
    ///     println!("Background task");
    /// });
    /// ```
    pub fn spawn<F: Future<Output = ()> + Send + 'static>(&self, fut: F) {
        let task = Task::new(fut, self.queue.clone());
        self.queue.push(task);
    }

    /// Blocks until the given future completes, processing spawned tasks along the way.
    ///
    /// Runs the provided future to completion, executing any spawned tasks from the queue
    /// when the main future is pending. Returns the output of the completed future.
    ///
    /// This method also establishes a runtime context, allowing tasks spawned within
    /// the future to use the global `spawn()` function without an explicit runtime reference.
    ///
    /// # Arguments
    /// * `fut` - The future to execute and wait for completion
    ///
    /// # Returns
    /// The output value of the completed future
    ///
    /// # Example
    /// ```ignore
    /// let result = rt.block_on(async { 42 });
    /// assert_eq!(result, 42);
    /// ```
    pub fn block_on<F: Future>(&self, fut: F) -> F::Output {
        enter_context(self.queue.clone(), || {
            let mut fut = Box::pin(fut);

            let w = noop_waker();
            let mut cx = Context::from_waker(&w);

            loop {
                match fut.as_mut().poll(&mut cx) {
                    Poll::Ready(val) => {
                        self.executor.run();
                        return val;
                    }
                    Poll::Pending => {
                        self.executor.run();
                    }
                }
            }
        })
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}
