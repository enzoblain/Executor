//! Runtime subsystem modules.
//!
//! Provides the core runtime infrastructure including task scheduling, execution,
//! and context management for asynchronous task execution.

pub(crate) mod context;
mod core;
pub(crate) mod driver;
pub(crate) mod executor;
pub(crate) mod queue;
pub(crate) mod waker;
pub mod yield_now;

pub(crate) use context::{CURRENT_QUEUE, enter_context};
pub use core::Runtime;
pub(crate) use executor::Executor;
pub(crate) use queue::TaskQueue;
pub(crate) use waker::make_waker;
