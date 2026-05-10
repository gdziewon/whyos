#![no_std]

mod arch;
mod scheduler;
mod task;
mod itc;
mod memory;
mod error;
mod utils;

pub use itc::{Mutex, Queue, Semaphore};
pub use task::{TaskBuilder, TaskInfo, StackSize};
pub use task::{TaskRoutine, TaskRoutineArg, TaskState, ResumeContext, TaskHandle};
pub use scheduler::MAX_TASKS;
pub use error::WhyError;

use core::num::NonZero;

use error::WhyResult;
use crate::scheduler::Kernel;
use crate::task::{TaskId};

// TODO: FIGURE OUT which ones are safe to call in MSP mode

/// Starts the WhyOS kernel and begins task scheduling.
///
/// # Panics
/// This function enforces strict one-shot initialization. It will `panic!` if
/// called more than once to prevent hardware and kernel state corruption.
///
/// # Arguments
/// * `freq` - The system tick frequency in Hertz (Hz).
pub fn start(freq: u32) -> ! {
    Kernel::init(freq);
    unsafe { arch::start_os(freq) }
}

#[inline] pub fn spawn(entry: TaskRoutine) -> WhyResult<TaskHandle> { TaskBuilder::new(entry).spawn() }
#[inline] pub fn spawn_with_priority(entry: TaskRoutine, priority: u8) -> WhyResult<TaskHandle> { TaskBuilder::new(entry).priority(priority).spawn() }

#[inline]
pub fn yield_cpu() {
    scheduler::yield_now();
}

#[inline]
pub fn sleep(ticks: u64) {
    if let Some(ticks) = NonZero::new(ticks) {
        let curr = current_tid();
        Kernel::lock(|k| {
            k.sleep_task(curr, ticks);
        });
    }

    scheduler::yield_now();
}

/// Terminates the current task and reclaims its memory.
///
/// **WARNING:** This function does NOT run destructors. If the task
/// holds a locked `Mutex` or other shared resources, they will remain
/// locked forever.
#[inline]
pub fn exit() -> ! {
    let curr = current_tid();
    Kernel::lock(|k| k.make_zombie(curr));
    scheduler::yield_now();
    loop {}
}

#[inline]
fn current_tid() -> TaskId { // todo: remove it maybe
    Kernel::lock(|k| k.current_task()
        .expect("WhyOS: no current task")
    )
}

pub fn my_handle() -> TaskHandle {
    let curr = current_tid();
    Kernel::lock(|k| {
        k.handle(curr).expect("WhyOS: Current task invalid")
    })
}

#[inline]
pub fn uptime_ticks() -> u64 {
    Kernel::lock(|k| k.system_ticks())
}

#[inline]
pub fn allocated() -> impl Iterator<Item = TaskHandle> {
    let map = Kernel::lock(|k| k.allocated());
    map.iter().filter_map(|tid| Kernel::lock(|k| k.handle(tid).ok()))
}

#[inline]
pub fn reclaim_memory() {
    Kernel::lock(|k| k.reap_zombies())
}

#[inline]
pub fn wdt_sub(interval_ticks: u64) {
    if let Some(ticks) = NonZero::new(interval_ticks) {
        let curr = current_tid();
        Kernel::lock(|k| {
            k.wdt_sub(curr, ticks);
        })
    }
}

#[inline]
pub fn wdt_unsub() {
    let curr = current_tid();
    Kernel::lock(|k| {
        k.wdt_unsub(curr);
    })
}

#[inline]
pub fn wdt_feed() {
    let curr = current_tid();
    Kernel::lock(|k| {
        k.wdt_feed(curr);
    })
}

#[inline]
pub fn reboot() -> ! {
    arch::reset();
}

#[inline]
pub const fn build_name() -> &'static str {
    env!("WHYOS_BUILD_IDENT")
}