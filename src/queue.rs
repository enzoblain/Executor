//! Thread-safe task queue for managing ready tasks.
//!
//! Provides a FIFO queue that allows pushing tasks to be executed and popping
//! them for execution by the executor.

use crate::task::Task;

use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// A thread-safe, FIFO queue for storing executable tasks.
///
/// Uses a Mutex-wrapped VecDeque to allow safe concurrent access from multiple threads.
/// Tasks are pushed when spawned and popped by the executor for execution.
pub(crate) struct TaskQueue {
    pub(crate) queue: Mutex<VecDeque<Arc<Task>>>,
}

impl TaskQueue {
    /// Creates a new empty task queue.
    ///
    /// Initializes an empty VecDeque wrapped in a Mutex for thread-safe access.
    pub(crate) fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
        }
    }

    /// Enqueues a task to be executed.
    ///
    /// Pushes the task to the back of the queue using FIFO (first-in-first-out) ordering.
    /// The task will be executed by the executor in the order it was added.
    ///
    /// # Arguments
    /// * `task` - The task to enqueue
    pub(crate) fn push(&self, task: Arc<Task>) {
        self.queue.lock().unwrap().push_back(task);
    }

    /// Dequeues and returns the next ready task.
    ///
    /// Removes and returns the task at the front of the queue if available.
    /// Returns None if the queue is empty.
    ///
    /// # Returns
    /// Some(task) if a task is available, None if the queue is empty
    pub(crate) fn pop(&self) -> Option<Arc<Task>> {
        self.queue.lock().unwrap().pop_front()
    }

    /// Checks if the task queue is empty.
    ///
    /// # Returns
    /// true if no tasks are queued, false otherwise
    pub(crate) fn is_empty(&self) -> bool {
        self.queue.lock().unwrap().is_empty()
    }
}
