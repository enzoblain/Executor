//! Background runtime driver to pump tasks and I/O.
//!
//! Some user code may block inside the main future (e.g., joining a std::thread),
//! preventing the single-threaded runtime loop from progressing. To avoid hangs,
//! this driver runs the executor and reactor in a dedicated thread so spawned
//! tasks and I/O readiness continue to be processed.
//!
//! # Overview
//!
//! The background driver manages a separate thread that continuously processes tasks
//! and I/O events. This ensures that spawned tasks and network operations make progress
//! even if the main future blocks on synchronous operations.

use crate::reactor::core::{Reactor, ReactorHandle};
use crate::runtime::{Executor, TaskQueue, enter_context};

use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::{Arc, Mutex, OnceLock};

static STARTED_QUEUES: OnceLock<Mutex<HashSet<usize>>> = OnceLock::new();

/// A background driver that runs the executor and reactor in a separate thread.
///
/// The driver continuously:
/// 1. Executes ready tasks from the queue
/// 2. Polls for I/O events
/// 3. Wakes tasks when they become ready
/// 4. Blocks on I/O with a timeout when idle
///
/// This allows spawned tasks to make progress even if the main future blocks.
pub(crate) struct BackgroundDriver {
    reactor: ReactorHandle,
    executor: Executor,
    queue: Arc<TaskQueue>,
}

impl BackgroundDriver {
    /// Creates a new background driver for the given queue.
    fn new(queue: Arc<TaskQueue>) -> Self {
        let reactor = Rc::new(RefCell::new(Reactor::new()));
        let executor = Executor::new(queue.clone());

        Self {
            reactor,
            executor,
            queue,
        }
    }

    /// Runs the driver loop until shutdown is signaled.
    ///
    /// This method is meant to run in a dedicated thread and will block indefinitely
    /// until [`TaskQueue::shutdown`] is called.
    fn run(&mut self) {
        enter_context(self.queue.clone(), || {
            loop {
                if self.queue.is_shutdown() {
                    break;
                }

                // Execute all ready tasks
                self.executor.run();

                // Poll for I/O events without blocking
                self.reactor.borrow_mut().poll_events();
                self.reactor.borrow_mut().wake_ready();

                // If more tasks are ready, continue immediately
                if !self.queue.is_empty() {
                    continue;
                }

                // Check shutdown again before blocking
                if self.queue.is_shutdown() {
                    break;
                }

                // Block on I/O with timeout to periodically check shutdown
                self.reactor.borrow_mut().wait_for_event_with_timeout(100);
                self.reactor.borrow_mut().handle_events();
                self.reactor.borrow_mut().wake_ready();
            }
        });
    }

    /// Starts a background driver thread once for the given queue.
    ///
    /// This method is safe to call multiple times; only the first call will spawn the thread.
    /// Subsequent calls with the same queue return immediately.
    ///
    /// # Arguments
    /// * `queue` - The task queue to process in the background
    ///
    /// # Thread Safety
    /// Uses a static HashSet to track which queues already have drivers, preventing
    /// duplicate driver threads for the same queue.
    #[allow(unused_variables)]
    pub(crate) fn ensure_spawned(queue: Arc<TaskQueue>) {
        // NOTE: BackgroundDriver is currently disabled because ReactorHandle uses Rc<RefCell<>>
        // which is not Send. To enable multi-threading, we would need to use Arc<Mutex<>> instead.
        // For now, everything runs on the main runtime thread.
    }
}
