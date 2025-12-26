//! Event-driven I/O reactor module.
//!
//! Provides the core event-driven I/O handling using kqueue on macOS.

pub mod core;
pub mod event;
pub mod future;
pub mod io;
pub mod sleep;
pub mod socket;
