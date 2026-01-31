#![no_std]

mod syscall;
mod scheduler;
mod task;
mod itc;
mod memory;
mod error;

pub use itc::{Mutex, Queue, Semaphore};
pub use task::{TaskId, TaskBuilder, TaskInfo, StackSize};
pub use task::{TaskRoutine, TaskState, ResumeContext};

use core::arch::asm;

use error::WhyResult;
use syscall::SvcNumber as SVC;

pub unsafe fn start(syst: &mut cortex_m::peripheral::SYST, freq: u32) -> ! { // todo: disable interrupts here?
    task::ops::init_idle_task();
    scheduler::config_systick(syst, freq);

    unsafe {
        asm!(
            "svc {ID}", ID = const SVC::Start.id(),
            options(noreturn)
        );
    }
}

#[inline] pub fn spawn(entry: TaskRoutine) -> WhyResult<TaskId> { TaskBuilder::new(entry).spawn() }
#[inline] pub fn spawn_with_priority(entry: TaskRoutine, priority: u8) -> WhyResult<TaskId> { TaskBuilder::new(entry).priority(priority).spawn() }

// todo: they all should probably return a Result
#[inline(always)]
pub fn yield_cpu() {
    unsafe {
        asm!(
            "svc {ID}",
            ID = const SVC::Yield.id()
        );
    }
}

#[inline(always)]
pub fn sleep(ticks: u64) {
    let low = ticks as u32;
    let high = (ticks >> 32) as u32;
    unsafe {
        asm!(
            "svc {ID}",
            ID = const SVC::Sleep.id(),
            in("r0") low,
            in("r1") high,
        );
    }
}

#[inline(always)]
pub fn exit() -> ! {
    unsafe {
        asm!(
            "svc {ID}",
            ID = const SVC::Exit.id(),
            options(noreturn)
        );
    }
}

#[inline(always)]
pub fn suspend(tid: TaskId) -> WhyResult<()> {
    let err: usize;
    unsafe {
        asm!(
            "svc {ID}",
            ID = const SVC::Suspend.id(),
            inout("r0") tid.0 => err,
        );
    }
    error::from_errno(err)
}

#[inline(always)]
pub fn resume(tid: TaskId) -> WhyResult<()> {
    let err: usize;
    unsafe {
        asm!(
            "svc {ID}",
            ID = const SVC::Resume.id(),
            inout("r0") tid.0 => err,
        );
    }
    error::from_errno(err)
}

#[inline(always)]
pub fn current_tid() -> TaskId {
    let tid: usize;
    unsafe {
        asm!(
            "svc {ID}",
            ID = const SVC::GetCurrentTid.id(),
            out("r0") tid,
        );
    }
    TaskId(tid)
}

#[inline] pub fn current_name() -> Option<&'static str> { syscall::get_current_name() }
#[inline] pub fn uptime_ticks() -> u64 { syscall::get_uptime_ticks() }
#[inline] pub fn task_count() -> usize { syscall::get_task_count() }
#[inline] pub fn task_info(tid: TaskId) -> WhyResult<TaskInfo> { syscall::get_task_info(tid) }
#[inline] pub fn active_tasks() -> impl Iterator<Item = TaskId> { syscall::get_active_tasks() }

#[inline] pub fn reclaim_memory() -> u8 { syscall::reclaim_memory() } // todo: do we need this?

#[inline] pub fn watchdog_subscribe(interval_ticks: u64) { syscall::watchdog_subscribe(interval_ticks) }
#[inline] pub fn watchdog_unsubscribe() { syscall::watchdog_unsubscribe() }
#[inline] pub fn watchdog_feed() { syscall::watchdog_feed() }