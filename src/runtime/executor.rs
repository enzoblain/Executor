//! Task executor that processes queued tasks.
//!
//! The executor is responsible for polling ready tasks from the task queue
//! until they complete or yield with Poll::Pending.

use super::queue::TaskQueue;

use std::sync::Arc;

/// Executes tasks from a shared task queue.
///
/// The executor continuously drains the task queue, polling each task until completion.
/// It is the core mechanism that allows concurrent execution of multiple futures.
pub(crate) struct Executor {
    pub(crate) queue: Arc<TaskQueue>,
}

impl Executor {
    /// Creates a new executor with the given task queue.
    ///
    /// # Arguments
    /// * `queue` - The task queue that the executor will drain
    pub fn new(queue: Arc<TaskQueue>) -> Self {
        Self { queue }
    }

    /// Executes all ready tasks in the queue until empty.
    ///
    /// Continuously polls tasks from the queue and runs them to completion or until
    /// they yield with Poll::Pending. The queue is drained completely before returning.
    pub fn run(&self) {
        while let Some(task) = self.queue.pop() {
            task.poll();
        }
    }
}
