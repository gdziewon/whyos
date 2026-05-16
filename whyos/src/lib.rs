//! **WhyOS** is a lightweight, fast, and safe [RTOS](https://en.wikipedia.org/wiki/Real-time_operating_system) designed for embedded Rust.
//!
//! It provides a preemptive, priority-based task scheduler with essential concurrency
//! primitives, enabling developers to build robust firmware with idiomatic Rust safety.
//!
//! # Key Features
//!
//! * **Preemptive Priority Scheduling:** Deterministic, priority-based multitasking ensuring critical tasks consistently meet their real-time deadlines.
//! * **ITC Primitives:** Ergonomic, priority-aware synchronization structures ([`Mutex`], [`Queue`], [`Semaphore`]).
//! * **Resilient By Design:** First-class support for software watchdogs, per-task panics and memory-safety at compile-time.
//!
//! # Basic Usage
//!
//! ```no_run
//! use whyos::{TaskBuilder, Freq};
//!
//! // Define a task routine
//! extern "C" fn blinky_task() {
//!     loop {
//!         // Toggle LED here...
//!         whyos::sleep(500); // Sleep for 500 ticks
//!     }
//! }
//!
//! fn main() -> ! {
//!     // 1. Initialize hardware (peripherals, clocks, etc.)
//!     // ...
//!
//!     // 2. Spawn tasks
//!     TaskBuilder::new(blinky_task)
//!         .name("blinky")
//!         .priority(10)
//!         .spawn()
//!         .expect("Failed to spawn blinky task");
//!
//!     // 3. Start the OS scheduler (this never returns)
//!     // We configure the system tick to 1 kHz (1 tick = 1 millisecond)
//!     whyos::start(Freq::ONE_KHZ);
//! }
//! ```
//!
#![no_std]

mod arch;
mod scheduler;
mod task;
mod itc;
mod memory;
mod error;
mod utils;
mod syscall;

pub use itc::{Mutex, MutexGuard, Queue, Semaphore};
pub use task::{TaskBuilder, TaskInfo, StackSize};
pub use task::{TaskRoutine, TaskRoutineArg, TaskState, ResumeContext, TaskHandle};
pub use scheduler::MAX_TASKS;
pub use error::WhyError;
pub use syscall::*;