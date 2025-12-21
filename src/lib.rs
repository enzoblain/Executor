//! Minimal async runtime implementation with task spawning and execution.
//!
//! This crate provides a basic asynchronous runtime that allows executing futures
//! and spawning concurrent tasks. It includes a task queue, executor, and waker system.
//!
//! # Architecture
//!
//! - **Runtime**: Main executor that runs futures to completion via `block_on`
//! - **Executor**: Processes queued tasks from the task queue
//! - **TaskQueue**: Thread-safe FIFO queue storing ready tasks
//! - **Task**: Wraps a future with waker integration
//! - **Waker**: Implements task wake-up mechanisms
//! - **RuntimeBuilder**: Fluent builder pattern for runtime instantiation
//! - **Timer**: Sleep futures for time-based delays and scheduling

mod builder;
mod context;
mod executor;
mod queue;
mod runtime;
pub mod task;
mod timer;
mod waker;

pub use builder::RuntimeBuilder;
pub use runtime::Runtime;
pub use timer::sleep;
