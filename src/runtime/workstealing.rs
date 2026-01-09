use crate::core::task::Runnable;
use crate::reactor::core::ReactorHandle;
use crate::runtime::context::{CURRENT_FEATURES, CURRENT_REACTOR, Features};

use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};

pub(crate) struct Injector {
    pub(crate) queue: Mutex<VecDeque<Arc<dyn Runnable>>>,
    pub(crate) condvar: Condvar,

    active: AtomicUsize,
    shutdown: AtomicBool,
}

impl Injector {
    pub(crate) fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            condvar: Condvar::new(),
            active: AtomicUsize::new(0),
            shutdown: AtomicBool::new(false),
        }
    }

    pub(crate) fn shutdown(&self) {
        self.shutdown.store(true, Ordering::Release);
        self.condvar.notify_all();
    }

    pub(crate) fn is_shutdown(&self) -> bool {
        self.shutdown.load(Ordering::Acquire)
    }

    pub(crate) fn push(&self, task: Arc<dyn Runnable>) {
        self.active.fetch_add(1, Ordering::Relaxed);

        let mut queue = self.queue.lock().unwrap();
        queue.push_back(task);

        self.condvar.notify_one();
    }

    pub(crate) fn reschedule(&self, task: Arc<dyn Runnable>) {
        let mut queue = self.queue.lock().unwrap();
        queue.push_back(task);

        self.condvar.notify_one();
    }

    pub(crate) fn pop(&self) -> Option<Arc<dyn Runnable>> {
        self.queue.lock().unwrap().pop_front()
    }

    pub(crate) fn task_completed(&self) {
        self.active.fetch_sub(1, Ordering::Release);
        self.condvar.notify_all();
    }

    pub(crate) fn is_idle(&self) -> bool {
        self.active.load(Ordering::Acquire) == 0
    }
}

pub(crate) struct LocalQueue {
    deque: Mutex<VecDeque<Arc<dyn Runnable>>>,
}

impl LocalQueue {
    pub(crate) fn new() -> Self {
        Self {
            deque: Mutex::new(VecDeque::new()),
        }
    }

    // pub fn push(&self, task: Arc<dyn Runnable>) {
    //     self.deque.lock().unwrap().push_back(task);
    // }

    pub(crate) fn pop(&self) -> Option<Arc<dyn Runnable>> {
        self.deque.lock().unwrap().pop_back()
    }

    pub(crate) fn steal(&self) -> Option<Arc<dyn Runnable>> {
        self.deque.lock().unwrap().pop_front()
    }
}

pub(crate) struct Worker {
    pub(crate) id: usize,
    pub(crate) locals: Arc<Vec<LocalQueue>>,
    pub(crate) injector: Arc<Injector>,
    pub(crate) reactor: ReactorHandle,
    pub(crate) features: Features,
}

impl Worker {
    pub fn run(&self) {
        CURRENT_INJECTOR.with(|cell| {
            *cell.borrow_mut() = Some(self.injector.clone());
        });
        CURRENT_REACTOR.with(|cell| {
            *cell.borrow_mut() = Some(self.reactor.clone());
        });
        CURRENT_FEATURES.with(|cell| {
            *cell.borrow_mut() = Some(self.features);
        });

        loop {
            if self.injector.is_shutdown() {
                return;
            }

            if let Some(task) = self.locals[self.id].pop() {
                task.poll();
            } else if let Some(task) = self.injector.pop() {
                task.poll();
            } else if let Some(task) = self.try_steal() {
                task.poll();
            } else {
                let queue = self.injector.queue.lock().unwrap();

                if queue.is_empty() && !self.injector.is_shutdown() {
                    let _ = self
                        .injector
                        .condvar
                        .wait_timeout(queue, std::time::Duration::from_millis(10));
                }
            }
        }
    }

    fn try_steal(&self) -> Option<Arc<dyn Runnable>> {
        let len = self.locals.len();

        for i in 0..len {
            let victim = (self.id + i + 1) % len;

            if let Some(task) = self.locals[victim].steal() {
                return Some(task);
            }
        }
        None
    }
}

thread_local! {
    pub static CURRENT_INJECTOR: RefCell<Option<Arc<Injector>>> = const { RefCell::new(None) };
}
