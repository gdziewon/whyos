#![no_std]

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

pub use cortex_m;
pub use cortex_m_rt;

use core::arch::asm;
use core::num::NonZero;

use error::WhyResult;
use crate::scheduler::{ContextSwitch, Kernel};
use crate::task::ops;

// TODO: FIGURE OUT which ones are safe to call in MSP mode


/// # Safety
/// Should only be called once by "main"
pub unsafe fn start(syst: &mut cortex_m::peripheral::SYST, freq: u32) -> ! { // todo: disable interrupts here?
    task::ops::init_idle_task(); // todo: they shouldn't be called here, but after the svc call
    scheduler::config_systick(syst, freq);

    unsafe { // bootstrap
        asm!(
            "svc 0",
            options(noreturn)
        );
    }
}

#[inline] pub fn spawn(entry: TaskRoutine) -> WhyResult<TaskId> { TaskBuilder::new(entry).spawn() }
#[inline] pub fn spawn_with_priority(entry: TaskRoutine, priority: u8) -> WhyResult<TaskId> { TaskBuilder::new(entry).priority(priority).spawn() }

// todo: they all should probably return a Result
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

/// # Safety
/// Calling this function will immediately terminate the task and reclaim its memory
/// however it will NOT run any "Drop" implementations for variables currently in scope
///
/// To ensure everything gets cleaned up, tasks should simply return from their entry point
#[inline]
pub unsafe fn exit() -> ! {
    let curr = current_tid();
    Kernel::lock(|k| k.make_zombie(curr).unwrap());
    scheduler::yield_now();
    loop {}
}

/// # Safety
/// Same as exit()
#[inline]
pub unsafe fn kill(tid: TaskId) -> WhyResult<()> {
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
    cortex_m::peripheral::SCB::sys_reset();
}