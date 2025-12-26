//! Minimal async runtime implementation with task spawning and execution.
//!
//! This crate provides a basic asynchronous runtime that allows executing futures
//! and spawning concurrent tasks. It includes a task queue, executor, and waker system.
//!
//! # Overview
//!
//! This is a lightweight, single-threaded async runtime designed for educational purposes
//! and demonstrating async/await patterns in Rust. It implements the core concepts needed
//! for executing async code:
//!
//! - **Task scheduling**: Futures are wrapped in tasks and scheduled for execution
//! - **Event-driven I/O**: Uses kqueue (macOS) for non-blocking I/O operations
//! - **Timer support**: Allows scheduling code execution after specific durations
//! - **Cooperative multitasking**: Tasks yield control when waiting for I/O or timers
//!
//! # Core Components
//!
//! ## Runtime
//! The [`Runtime`] is the main entry point. It executes futures via [`block_on`](Runtime::block_on)
//! and manages task scheduling:
//!
//! ```ignore
//! let mut runtime = Runtime::new();
//! runtime.block_on(async {
//!     println!("Running async code");
//! });
//! ```
//!
//! ## Task Spawning
//! Spawn background tasks using [`Task::spawn`] from within an async context:
//!
//! ```ignore
//! #[tokio::main]
//! async fn main() {
//!     Task::spawn(async {
//!         println!("Background task");
//!     });
//! }
//! ```
//!
//! ## I/O Operations
//! The runtime provides non-blocking TCP networking via [`TcpListener`](net::tcp_listener::TcpListener)
//! and [`TcpStream`](net::tcp_stream::TcpStream):
//!
//! ```ignore
//! use reactor::net::tcp_listener::TcpListener;
//!
//! async fn server() {
//!     let listener = TcpListener::bind("127.0.0.1:8080").await.unwrap();
//!     loop {
//!         let (stream, addr) = listener.accept().await.unwrap();
//!         println!("New connection from {}", addr);
//!     }
//! }
//! ```
//!
//! ## Async Primitives
//! ### Sleep
//! Delay execution for a specified duration:
//!
//! ```ignore
//! use std::time::Duration;
//! use reactor::sleep;
//!
//! async fn delayed() {
//!     sleep(Duration::from_secs(1)).await;
//!     println!("1 second later");
//! }
//! ```
//!
//! ### Yield
//! Yield control to other tasks:
//!
//! ```ignore
//! use reactor::yield_now;
//!
//! async fn cooperative() {
//!     yield_now().await;
//!     println!("Other tasks had a chance to run");
//! }
//! ```
//!
//! # Architecture
//!
//! ## Internal Design
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────┐
//! │                    Runtime (block_on)                   │
//! │  Polls main future and spawned tasks until completion   │
//! └──────────┬──────────────────────────────────────────────┘
//!            │
//!            ├─► Executor: Drains task queue
//!            │   └─► Task::poll: Polls futures with custom waker
//!            │
//!            └─► Reactor: Manages I/O events
//!                ├─► Registers file descriptors
//!                ├─► Waits for events (kqueue)
//!                └─► Wakes tasks when ready
//! ```
//!
//! ### Key Components
//!
//! - **Runtime**: Main event loop that orchestrates everything
//! - **Executor**: Processes ready tasks from the task queue
//! - **TaskQueue**: Thread-safe FIFO queue storing [`Task`] instances
//! - **Task**: Wraps a future and manages waker integration
//! - **Waker**: Implements Rust's task waking protocol to re-queue tasks
//! - **Reactor**: Event loop for I/O and timer management (uses kqueue on macOS)
//! - **RuntimeBuilder**: Builder pattern for runtime instantiation
//!
//! ## Execution Flow
//!
//! 1. User calls `runtime.block_on(async_block)`
//! 2. Runtime enters main loop:
//!    - Poll main future
//!    - Drain task queue (execute all ready tasks)
//!    - Check reactor for I/O events
//!    - When idle, block on I/O (kevent) until events or timeout
//! 3. When a task is woken (I/O ready or timer fired), it's re-queued
//! 4. When main future completes, runtime returns its value
//!
//! # Features
//!
//! ## Non-blocking I/O
//! TCP operations never block the runtime. File descriptors are set to non-blocking mode,
//! and the reactor uses kqueue to efficiently wait for readiness on multiple file descriptors.
//!
//! ## Timers
//! Timers are implemented via kqueue's EVFILT_TIMER filter, allowing precise scheduling
//! without busy-waiting.
//!
//! ## Task Spawning
//! Tasks can spawn other tasks using [`Task::spawn`], which is safe to call from within
//! any async function running on the runtime.
//!
//! ## Background Driver
//! To handle blocking operations gracefully, a background driver thread is spawned when needed,
//! ensuring that I/O and timers continue to progress even if the main future blocks.
//!
//! # Limitations
//!
//! - **Single-threaded**: All task execution happens on a single thread
//! - **macOS only**: Uses kqueue, which is not available on Linux/Windows
//! - **No preemption**: Tasks are not preempted; they must yield explicitly
//! - **Minimal**: This is a simplified implementation for learning purposes
//!
//! # Thread Safety
//!
//! The runtime is **not thread-safe**. All operations must happen on the same thread where
//! the runtime was created. However:
//!
//! - The background driver thread can spawn additional I/O operations
//! - The task queue is protected by a Mutex for safe access from the driver thread
//!
//! # Example: Simple Echo Server
//!
//! ```ignore
//! use reactor::{Runtime, net::tcp_listener::TcpListener};
//! use std::time::Duration;
//!
//! #[test]
//! fn echo_server() {
//!     let mut rt = Runtime::new();
//!
//!     rt.block_on(async {
//!         let listener = TcpListener::bind("127.0.0.1:9999").await.unwrap();
//!         let addr = listener.local_addr().unwrap();
//!         println!("Listening on {}", addr);
//!
//!         loop {
//!             let (stream, peer_addr) = listener.accept().await.unwrap();
//!             println!("New connection from {}", peer_addr);
//!
//!             let mut buf = [0u8; 1024];
//!             match stream.read(&mut buf).await {
//!                 Ok(n) if n > 0 => {
//!                     println!("Received: {:?}", &buf[..n]);
//!                     stream.write_all(&buf[..n]).await.ok();
//!                 }
//!                 _ => break,
//!             }
//!         }
//!     });
//! }
//! ```

mod builder;

pub mod net;

mod reactor;
mod runtime;
mod task;

pub use builder::RuntimeBuilder;
pub use reactor::future::{AsyncRead, AsyncWrite};
pub use reactor::sleep::sleep;
pub use runtime::Runtime;
pub use runtime::yield_now::yield_now;
pub use task::{JoinHandle, JoinSet, Task};
