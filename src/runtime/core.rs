//! Async runtime that executes futures and manages task scheduling.
//!
//! The runtime coordinates the execution of a main future via `block_on` and handles
//! spawned background tasks. It uses a task queue and executor to manage concurrent execution.
//!
//! # Main Event Loop
//!
//! The runtime's `block_on` method implements the main event loop:
//! 1. Polls the main future
//! 2. Executes all ready tasks from the queue
//! 3. Polls for I/O events using the reactor
//! 4. Blocks on I/O when idle, or continues if more tasks are ready
//!
//! # Usage
//!
//! ```ignore
//! let mut rt = Runtime::new();
//! let result = rt.block_on(async {
//!     println!("Hello from async");
//!     42
//! });
//! assert_eq!(result, 42);
//! ```
//!
//! # Context Management
//!
//! The runtime establishes a thread-local context that allows spawned tasks to use
//! [`Task::spawn`] without an explicit runtime reference. This enables patterns similar
//! to `tokio::spawn`.
//!
//! [`Task::spawn`]: crate::task::Task::spawn

use crate::reactor::core::{Reactor, ReactorHandle};
use crate::runtime::{Executor, TaskQueue, enter_context};
use crate::task::Task;

use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;
use std::task::{Context, Poll};

/// Main async runtime for executing futures.
///
/// Provides the core API for running futures to completion and spawning background tasks.
/// The runtime maintains an executor and task queue for managing concurrent task execution.
pub struct Runtime {
    queue: Rc<TaskQueue>,
    executor: Executor,
    reactor: ReactorHandle,
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
        let queue = Rc::new(TaskQueue::new());
        let executor = Executor::new(queue.clone());
        let reactor = Rc::new(RefCell::new(Reactor::new()));

        Self {
            queue,
            executor,
            reactor,
        }
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
    pub fn spawn<F: Future<Output = ()> + 'static>(&self, fut: F) {
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
    pub fn block_on<F: Future>(&mut self, fut: F) -> F::Output {
        enter_context(self.queue.clone(), self.reactor.clone(), || {
            let mut fut = Box::pin(fut);

            let mut notified = false;

            fn clone(ptr: *const ()) -> std::task::RawWaker {
                std::task::RawWaker::new(ptr, &VTABLE)
            }

            fn wake(ptr: *const ()) {
                unsafe {
                    *(ptr as *mut bool) = true;
                }
            }

            fn wake_by_ref(ptr: *const ()) {
                unsafe {
                    *(ptr as *mut bool) = true;
                }
            }

            fn drop(_: *const ()) {}

            static VTABLE: std::task::RawWakerVTable =
                std::task::RawWakerVTable::new(clone, wake, wake_by_ref, drop);

            let raw = std::task::RawWaker::new(&mut notified as *mut bool as *const (), &VTABLE);
            let w = unsafe { std::task::Waker::from_raw(raw) };
            let mut cx = Context::from_waker(&w);

            loop {
                if let Poll::Ready(val) = fut.as_mut().poll(&mut cx) {
                    for _ in 0..10 {
                        self.executor.run();
                        if self.queue.is_empty() {
                            break;
                        }
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }

                    return val;
                }

                self.executor.run();

                self.reactor.borrow_mut().poll_events();
                self.reactor.borrow_mut().wake_ready();

                if notified {
                    notified = false;
                    continue;
                }

                if !self.queue.is_empty() {
                    continue;
                }

                self.reactor.borrow_mut().wait_for_event();
                self.reactor.borrow_mut().handle_events();
                self.reactor.borrow_mut().wake_ready();
            }
        })
    }

    /// Returns a handle to the reactor for use by futures.
    ///
    /// This allows futures to register I/O and timer events with the reactor.
    pub fn reactor_handle(&self) -> ReactorHandle {
        self.reactor.clone()
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}
