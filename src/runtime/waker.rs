use crate::Task;

use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::task::{RawWaker, RawWakerVTable, Waker};

pub(crate) struct TaskWaker<T: Send + Sync + 'static> {
    task: Arc<Task<T>>,
}

impl<T: Send + Sync + 'static> TaskWaker<T> {
    pub(crate) fn new(task: Arc<Task<T>>) -> Arc<Self> {
        Arc::new(Self { task })
    }

    fn wake(self: &Arc<Self>) {
        if !self.task.inqueue.swap(true, Ordering::AcqRel) {
            self.task.injector.reschedule(self.task.clone());
        }
    }

    fn clone_raw(data_ptr: *const ()) -> RawWaker {
        unsafe {
            let arc = Arc::<TaskWaker<T>>::from_raw(data_ptr as *const TaskWaker<T>);
            let cloned = arc.clone();

            std::mem::forget(arc);

            RawWaker::new(Arc::into_raw(cloned) as *const (), &Self::VTABLE)
        }
    }

    fn wake_raw(data_ptr: *const ()) {
        unsafe {
            let arc = Arc::<TaskWaker<T>>::from_raw(data_ptr as *const TaskWaker<T>);
            arc.wake();
        }
    }

    fn wake_by_ref_raw(data_ptr: *const ()) {
        unsafe {
            let arc = Arc::<TaskWaker<T>>::from_raw(data_ptr as *const TaskWaker<T>);
            arc.wake();
            let _ = Arc::into_raw(arc);
        }
    }

    fn drop_raw(data_ptr: *const ()) {
        unsafe {
            Arc::<TaskWaker<T>>::from_raw(data_ptr as *const TaskWaker<T>);
        }
    }

    pub const VTABLE: RawWakerVTable = RawWakerVTable::new(
        Self::clone_raw,
        Self::wake_raw,
        Self::wake_by_ref_raw,
        Self::drop_raw,
    );
}

pub(crate) fn make_waker<T: Send + Sync + 'static>(task: Arc<Task<T>>) -> Waker {
    let task_waker = TaskWaker::new(task);
    let raw_waker = RawWaker::new(
        Arc::into_raw(task_waker) as *const (),
        &TaskWaker::<T>::VTABLE,
    );

    unsafe { Waker::from_raw(raw_waker) }
}
