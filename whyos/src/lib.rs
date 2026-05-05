#![no_std]

mod arch;
mod scheduler;
mod task;
mod itc;
mod memory;
mod error;
mod utils;

pub use itc::{Mutex, Queue, Semaphore};
pub use task::{TaskId, TaskBuilder, TaskInfo, StackSize};
pub use task::{TaskRoutine, TaskRoutineArg, TaskState, ResumeContext};
pub use scheduler::MAX_TASKS;
pub use error::WhyError;

use core::num::NonZero;

use error::WhyResult;
use crate::scheduler::{ContextSwitch, Kernel};
use crate::task::ops;

// TODO: FIGURE OUT which ones are safe to call in MSP mode
// todov2: implement task handle


/// Starts the WhyOS kernel and begins task scheduling.
///
/// # Panics
/// This function enforces strict one-shot initialization. It will `panic!` if
/// called more than once to prevent hardware and kernel state corruption.
///
/// # Arguments
/// * `freq` - The system tick frequency in Hertz (Hz).
pub fn start(freq: u32) -> ! {
    Kernel::init();
    unsafe { arch::start_os(freq) }
}

#[inline] pub fn spawn(entry: TaskRoutine) -> WhyResult<TaskId> { TaskBuilder::new(entry).spawn() }
#[inline] pub fn spawn_with_priority(entry: TaskRoutine, priority: u8) -> WhyResult<TaskId> { TaskBuilder::new(entry).priority(priority).spawn() }

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
    Kernel::lock(|k| k.make_zombie(curr).unwrap());
    scheduler::yield_now();
    loop {}
}

/// Immediately kills the specified task and reclaims its memory.
///
/// **WARNING:** This function does NOT run destructors. If the task
/// holds a locked `Mutex` or other shared resources, they will remain
/// locked forever.
#[inline]
pub fn kill(tid: TaskId) -> WhyResult<()> {
    let switch = Kernel::lock(|k| k.make_zombie(tid))?;
    if switch == ContextSwitch::Yield {
        scheduler::yield_now();
    }
    Ok(())
}

#[inline]
pub fn suspend(tid: TaskId) -> WhyResult<()> {
    let switch = Kernel::lock(|k| k.suspend_task(tid))?;
    if switch == ContextSwitch::Yield {
        scheduler::yield_now();
    }
    Ok(())
}

#[inline]
pub fn resume(tid: TaskId) -> WhyResult<()> {
    let switch = Kernel::lock(|k| k.resume_task(tid))?;
    if switch == ContextSwitch::Yield {
        scheduler::yield_now();
    }
    Ok(())
}

#[inline]
pub fn current_tid() -> TaskId {
    Kernel::lock(|k| k.current_task()
        .expect("WhyOS: no current task")
    )
}

#[inline]
pub fn uptime_ticks() -> u64 {
    Kernel::lock(|k| k.system_ticks())
}

#[inline]
pub fn task_count() -> usize {
    Kernel::lock(|k| k.allocated().ones())
}

#[inline]
pub fn allocated_tasks() -> impl Iterator<Item = TaskId> {
    Kernel::lock(|k| k.allocated().iter())
}

#[inline]
pub fn task_info(tid: TaskId) -> WhyResult<TaskInfo> {
    Kernel::lock(|k| {
        if !k.allocated().is_set(tid) {
            return Err(WhyError::InvalidTaskId);
        }

        let task = k.task(tid);
        TaskInfo::new(tid, task)
    })
}

#[inline]
pub fn reclaim_memory() -> usize {
    ops::reap_zombies()
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