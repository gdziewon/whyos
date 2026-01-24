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

use error::{WhyResult};
use task::ops;

pub unsafe fn start(syst: &mut cortex_m::peripheral::SYST, freq: u32) -> ! { // todo: disable interrupts here?
    ops::init_idle_task();
    scheduler::config_systick(syst, freq);

    unsafe {
        core::arch::asm!("svc 0", options(noreturn));
    }
}

#[inline] pub fn spawn(entry: TaskRoutine) -> WhyResult<TaskId> { TaskBuilder::new(entry).spawn() }
#[inline] pub fn spawn_with_priority(entry: TaskRoutine, priority: u8) -> WhyResult<TaskId> { TaskBuilder::new(entry).priority(priority).spawn() }

#[inline] pub fn yield_cpu() { syscall::yield_now(); }
#[inline] pub fn sleep(ticks: u64) { syscall::sleep(ticks); }
#[inline] pub fn exit() -> ! { ops::exit() }
#[inline] pub fn suspend(tid: TaskId) -> WhyResult<()> { syscall::suspend(tid) }
#[inline] pub fn resume(tid: TaskId) -> WhyResult<()> { syscall::resume(tid) }


#[inline] pub fn current_tid() -> TaskId { syscall::current_tid() }
#[inline] pub fn current_name() -> Option<&'static str> { syscall::current_name() }
#[inline] pub fn uptime_ticks() -> u64 { syscall::uptime_ticks() }
#[inline] pub fn task_count() -> usize { syscall::task_count() }
#[inline] pub fn reclaim_memory() { ops::reap_zombies(); }
#[inline] pub fn task_info(tid: TaskId) -> WhyResult<TaskInfo> { syscall::get_task_info(tid) }
#[inline] pub fn active_tasks() -> impl Iterator<Item = TaskId> { syscall::active_tasks() }