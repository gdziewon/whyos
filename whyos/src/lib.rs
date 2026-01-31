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
use core::mem::MaybeUninit;

use error::WhyResult;
use syscall::SvcNumber as SVC;

use crate::task::TaskMap;

/// # Safety
/// Should only be called once by "main"
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
#[inline]
pub fn yield_cpu() {
    unsafe {
        asm!(
            "svc {ID}",
            ID = const SVC::Yield.id()
        );
    }
}

#[inline]
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

#[inline]
pub fn exit() -> ! {
    unsafe {
        asm!(
            "svc {ID}",
            ID = const SVC::Exit.id(),
            options(noreturn)
        );
    }
}

#[inline]
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

#[inline]
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

#[inline]
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

#[inline]
pub fn current_name() -> Option<&'static str> {
    let ptr: usize;
    let len: usize;
    unsafe {
        asm!(
            "svc {ID}",
            ID = const SVC::GetCurrentName.id(),
            out("r0") ptr,
            out("r1") len,
        );
    }

    let ptr = ptr as *const u8;
    if ptr.is_null() {
        None
    } else {
        Some(unsafe {
            core::str::from_utf8_unchecked(
                core::slice::from_raw_parts(ptr, len)
            )
        })
    }
}

#[inline]
pub fn uptime_ticks() -> u64 {
    let ticks_low: usize;
    let ticks_high: usize;
    unsafe {
        asm!(
            "svc {ID}",
            ID = const SVC::GetUptimeTicks.id(),
            out("r0") ticks_low,
            out("r1") ticks_high,
        );
    }
    (ticks_low as u64) | (ticks_high as u64) << 32
}

#[inline]
pub fn task_count() -> usize {
    let task_count: usize;
    unsafe {
        asm!(
            "svc {ID}",
            ID = const SVC::GetTaskCount.id(),
            out("r0") task_count,
        );
    }
    task_count
}

#[inline]
pub fn task_info(tid: TaskId) -> WhyResult<TaskInfo> {
    let err: usize;
    let mut task_info = MaybeUninit::<TaskInfo>::uninit();
    unsafe {
        asm!(
            "svc {ID}",
            ID = const SVC::GetTaskInfo.id(),
            inout("r0") tid.0 => err,
            in("r1") task_info.as_mut_ptr()
        );
    }

    if let Err(e) = error::from_errno(err) {
        Err(e)
    } else {
        Ok(unsafe { task_info.assume_init() })
    }
}

#[inline]
pub fn active_tasks() -> impl Iterator<Item = TaskId> {
    let active_tasks: usize;
    unsafe {
        asm!(
            "svc {ID}",
            ID = const SVC::GetActiveTasks.id(),
            out("r0") active_tasks,
        );
    }

    TaskMap::from(active_tasks as u32)
        .iter()
        .map(TaskId)
}

#[inline]
pub fn reclaim_memory() -> usize {
    let reclaimed: usize;
    unsafe {
        asm!(
            "svc {ID}",
            ID = const SVC::ReclaimMemory.id(),
            out("r0") reclaimed,
        );
    }
    reclaimed
}

#[inline]
pub fn watchdog_subscribe(interval_ticks: u64) {
    let low = interval_ticks as u32;
    let high = (interval_ticks >> 32) as u32;
    unsafe {
        asm!(
            "svc {ID}",
            ID = const SVC::WatchdogSubscribe.id(),
            in("r0") low,
            in("r1") high,
        );
    }
}

#[inline]
pub fn watchdog_unsubscribe() {
    unsafe {
        asm!(
            "svc {ID}",
            ID = const SVC::WatchdogUnsubscribe.id(),
        );
    }
}

#[inline]
pub fn watchdog_feed() {
    unsafe {
        asm!(
            "svc {ID}",
            ID = const SVC::WatchdogFeed.id(),
        );
    }
}