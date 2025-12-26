//! Waker implementation for task wake-up notifications.
//!
//! Provides task waker objects that notify the executor when a task is ready to continue.
//! Implements the standard Rust task waking protocol using RawWaker and RawWakerVTable.

use crate::task::Task;

use std::rc::Rc;
use std::task::{RawWaker, RawWakerVTable, Waker};

/// Custom waker that re-queues tasks when awakened.
///
/// Implements the Rust waker protocol to automatically re-enqueue a task
/// when it becomes ready to make further progress.
pub struct TaskWaker {
    task: Rc<Task>,
}

impl TaskWaker {
    /// Creates a new waker for the given task.
    ///
    /// # Arguments
    /// * `task` - The task to wake when notified
    ///
    /// # Returns
    /// An Rc-wrapped TaskWaker
    pub fn new(task: Rc<Task>) -> Rc<Self> {
        Rc::new(Self { task })
    }

    /// Wakes the task by re-enqueueing it.
    fn wake(self: &Rc<Self>) {
        self.task.queue.push(self.task.clone());
    }

    /// Raw waker clone function for RawWakerVTable.
    fn clone_raw(ptr: *const ()) -> RawWaker {
        unsafe {
            let rc = Rc::<TaskWaker>::from_raw(ptr as *const TaskWaker);
            let cloned = rc.clone();
            std::mem::forget(rc);
            RawWaker::new(Rc::into_raw(cloned) as *const (), &Self::VTABLE)
        }
    }

    /// Raw waker wake function for RawWakerVTable.
    fn wake_raw(ptr: *const ()) {
        unsafe {
            let rc = Rc::<TaskWaker>::from_raw(ptr as *const TaskWaker);
            rc.wake();
        }
    }

    /// Raw waker wake-by-reference function for RawWakerVTable.
    fn wake_by_ref_raw(ptr: *const ()) {
        unsafe {
            let rc = Rc::<TaskWaker>::from_raw(ptr as *const TaskWaker);
            rc.wake();
            let _ = Rc::into_raw(rc);
        }
    }

    /// Raw waker drop function for RawWakerVTable.
    fn drop_raw(ptr: *const ()) {
        unsafe {
            Rc::<TaskWaker>::from_raw(ptr as *const TaskWaker);
        }
    }

    /// Virtual method table for raw waker operations.
    pub const VTABLE: RawWakerVTable = RawWakerVTable::new(
        Self::clone_raw,
        Self::wake_raw,
        Self::wake_by_ref_raw,
        Self::drop_raw,
    );
}

/// Creates a Waker from a Task that re-queues on wake.
///
/// Constructs a Waker that implements the standard Rust task waking protocol.
/// When woken, the task is pushed back to the queue for re-execution.
///
/// # Arguments
/// * `task` - The task to create a waker for
///
/// # Returns
/// A Waker that will re-queue the task when called
pub(crate) fn make_waker(task: Rc<Task>) -> Waker {
    let task_waker = TaskWaker::new(task);
    let raw = RawWaker::new(Rc::into_raw(task_waker) as *const (), &TaskWaker::VTABLE);
    unsafe { Waker::from_raw(raw) }
}
