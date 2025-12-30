//! Time utilities: async sleep, timeout, and task timing.
//!
//! This module provides time-related async primitives:
//!
//! - [`sleep()`] for non-blocking delays
//! - [`timeout()`] for running a future with a deadline
//! - [`wrapper::Time`] for measuring elapsed time of async tasks
//!
//! # Example: Sleep
//!
//! ```ignore
//! use reactor::time::sleep;
//! use std::time::Duration;
//!
//! async fn wait() {
//!     sleep(Duration::from_secs(1)).await;
//! }
//! ```
//!
//! # Example: Timeout
//!
//! ```ignore
//! use reactor::time::{timeout, sleep};
//! use std::time::Duration;
//!
//! async fn run_with_timeout() {
//!     let result = timeout(Duration::from_millis(100), async {
//!         sleep(Duration::from_millis(50)).await;
//!         "done"
//!     }).await;
//!     assert_eq!(result, Ok("done"));
//! }
//! ```
//!
//! # Example: Timing a Task
//!
//! ```ignore
//! use reactor::time::{sleep, wrapper::Time};
//! use reactor::Task;
//! use std::time::Duration;
//!
//! async fn timed() {
//!     let handle = Task::spawn(async { sleep(Duration::from_millis(10)).await; });
//!     let (_result, elapsed) = Time::new(handle).await;
//!     println!("Elapsed: {:?}", elapsed);
//! }
//! ```
//!
//! # Errors
//!
//! The [`TimeError`] enum is returned by [`timeout()`] when the deadline is exceeded.
//!
//! ```ignore
//! use reactor::time::{timeout, sleep, TimeError};
//! use std::time::Duration;
//!
//! async fn timeout_expires() {
//!     let result = timeout(Duration::from_millis(10), async {
//!         sleep(Duration::from_millis(100)).await;
//!     }).await;
//!     assert!(matches!(result, Err(TimeError::TimeOut)));
//! }
//! ```

pub mod sleep;
pub mod timeout;
pub mod wrapper;

pub use sleep::sleep;
pub use timeout::timeout;

pub enum TimeError {
    TimeOut,
}
