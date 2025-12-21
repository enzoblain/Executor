//! Global runtime context for thread-local task spawning.
//!
//! Provides a thread-local runtime handle that allows spawning tasks without
//! explicitly passing a runtime reference, similar to tokio::spawn.

use crate::queue::TaskQueue;
use crate::task::Task;

use std::cell::RefCell;
use std::future::Future;
use std::sync::Arc;

thread_local! {
    /// Thread-local storage for the current runtime's task queue.
    ///
    /// When a runtime is entered via `block_on`, its task queue is stored here,
    /// allowing `spawn()` to work without an explicit runtime reference.
    static CURRENT_QUEUE: RefCell<Option<Arc<TaskQueue>>> = const { RefCell::new(None) };
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

/// Spawns a task on the current runtime context.
///
/// This function allows spawning tasks without holding a runtime reference,
/// similar to `tokio::spawn()`. Must be called from within a runtime context
/// (i.e., from within a `block_on` or spawned task).
///
/// # Arguments
/// * `fut` - The future to spawn as a background task
///
/// # Panics
/// Panics if called outside of a runtime context (no runtime is active on this thread).
///
/// # Example
/// ```ignore
/// use r#async::{Runtime, spawn};
///
/// let rt = Runtime::new();
/// rt.block_on(async {
///     spawn(async {
///         println!("Spawned task");
///     });
/// });
/// ```
pub fn spawn<F: Future<Output = ()> + Send + 'static>(fut: F) {
    CURRENT_QUEUE.with(|current| {
        let queue = current
            .borrow()
            .as_ref()
            .expect("spawn() called outside of a runtime context")
            .clone();

        let task = Task::new(fut, queue.clone());
        queue.push(task);
    });
}
