//! Global runtime context for thread-local task spawning.
//!
//! Provides a thread-local runtime handle that allows spawning tasks without
//! explicitly passing a runtime reference, similar to tokio::spawn.

use super::queue::TaskQueue;
use crate::reactor::core::ReactorHandle;

use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    /// Thread-local storage for the current runtime's task queue.
    ///
    /// When a runtime is entered via `block_on`, its task queue is stored here,
    /// allowing `Task::spawn()` to work without an explicit runtime reference.
    /// This enables global task spawning similar to tokio::spawn().
    pub(crate) static CURRENT_QUEUE: RefCell<Option<Rc<TaskQueue>>> =
        const { RefCell::new(None) };

    /// Thread-local storage for the current runtime's reactor handle.
    ///
    /// When a runtime is entered via `block_on`, its reactor handle is stored here,
    /// allowing I/O operations to work without an explicit reactor reference.
    /// This enables patterns like `TcpListener::bind()` without passing a reactor.
    pub(crate) static CURRENT_REACTOR: RefCell<Option<ReactorHandle>> =
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
pub(crate) fn enter_context<F, R>(queue: Rc<TaskQueue>, reactor: ReactorHandle, f: F) -> R
where
    F: FnOnce() -> R,
{
    CURRENT_QUEUE.with(|current_queue| {
        CURRENT_REACTOR.with(|current_reactor| {
            let previous_queue = current_queue.borrow_mut().replace(queue.clone());
            let previous_reactor = current_reactor.borrow_mut().replace(reactor.clone());

            let result = f();

            *current_queue.borrow_mut() = previous_queue;
            *current_reactor.borrow_mut() = previous_reactor;

            result
        })
    })
}

/// Gets the current reactor handle from the thread-local context.
///
/// This is used internally by I/O operations like `TcpListener::bind()`
/// to automatically use the current runtime's reactor without requiring
/// an explicit reactor parameter.
///
/// # Returns
/// The current reactor handle if inside a runtime context, or `None` if called
/// outside of a runtime context (i.e., not within a `block_on` call).
///
/// # Panics
/// Panics if called outside a runtime context. All I/O operations should be
/// performed within a `Runtime::block_on` call.
pub fn current_reactor() -> ReactorHandle {
    CURRENT_REACTOR.with(|current| {
        current.borrow().clone().expect(
            "No reactor in current context. I/O operations must be called within Runtime::block_on",
        )
    })
}
