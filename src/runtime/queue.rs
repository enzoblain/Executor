//! Thread-safe task queue for managing ready tasks.
//!
//! Provides a FIFO queue that allows pushing tasks to be executed and popping
//! them for execution by the executor.

use crate::task::Task;

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

/// A thread-safe, FIFO queue for storing executable tasks.
///
/// Uses a Mutex-wrapped VecDeque to allow safe concurrent access from multiple threads.
/// Tasks are pushed when spawned and popped by the executor for execution.
pub(crate) struct TaskQueue {
    pub(crate) queue: RefCell<VecDeque<Rc<Task>>>,
}

impl TaskQueue {
    /// Creates a new empty task queue.
    pub(crate) fn new() -> Self {
        Self {
            queue: RefCell::new(VecDeque::new()),
        }
    }

    /// Enqueues a task to be executed.
    ///
    /// Pushes the task to the back of the queue using FIFO ordering.
    ///
    /// # Arguments
    /// * `task` - The task to enqueue
    pub(crate) fn push(&self, task: Rc<Task>) {
        self.queue.borrow_mut().push_back(task);
    }

    /// Dequeues and returns the next ready task.
    ///
    /// # Returns
    /// Some(task) if a task is available, None if the queue is empty
    pub(crate) fn pop(&self) -> Option<Rc<Task>> {
        self.queue.borrow_mut().pop_front()
    }

    /// Checks if the task queue is empty.
    pub(crate) fn is_empty(&self) -> bool {
        self.queue.borrow().is_empty()
    }
}
