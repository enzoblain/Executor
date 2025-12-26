//! Global runtime context for thread-local task spawning.
//!
//! Provides a thread-local runtime handle that allows spawning tasks without
//! explicitly passing a runtime reference, similar to tokio::spawn.

use super::queue::TaskQueue;

use std::cell::RefCell;
use std::sync::Arc;

thread_local! {
    /// Thread-local storage for the current runtime's task queue.
    ///
    /// When a runtime is entered via `block_on`, its task queue is stored here,
    /// allowing `Task::spawn()` to work without an explicit runtime reference.
    /// This enables global task spawning similar to tokio::spawn().
    pub(crate) static CURRENT_QUEUE: RefCell<Option<Arc<TaskQueue>>> =
        const { RefCell::new(None) };
}

/// Enters a runtime context with the given task queue.
///
/// Sets the thread-local runtime context so that `spawn()` can work without
/// an explicit runtime handle.
///
/// # Arguments
/// * `queue` - The task queue to use as the current runtime context
/// * `f` - The function to execute within this runtime context
///
/// # Returns
/// The return value of the function `f`
pub(crate) fn enter_context<F, R>(queue: Arc<TaskQueue>, f: F) -> R
where
    F: FnOnce() -> R,
{
    CURRENT_QUEUE.with(|current| {
        let previous = current.borrow_mut().replace(queue.clone());
        let result = f();
        *current.borrow_mut() = previous;

        result
    })
}
