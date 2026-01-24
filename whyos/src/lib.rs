#![no_std]

mod scheduler;
mod task;
mod itc;
mod memory;
mod error;

pub use itc::{Mutex, Queue, Semaphore};
pub use task::{TaskId, TaskBuilder, TaskInfo, StackSize};
pub use task::{TaskRoutine, TaskState, ResumeContext};

use error::{WhyError, WhyResult};
use scheduler::{KERNEL, IDLE_TID};

#[inline]
pub fn spawn(entry: TaskRoutine) -> WhyResult<TaskId> {
    TaskBuilder::new(entry).spawn()
}

#[inline]
pub fn spawn_with_priority(entry: TaskRoutine, priority: u8) -> WhyResult<TaskId> {
    TaskBuilder::new(entry).priority(priority).spawn()
}

#[inline]
pub fn sleep(ticks: u64) {
    if ticks == 0 {
        scheduler::yield_now(); // just yield, dont sleep
        return;
    }

    critical_section::with(|cs| {
        let mut kernel = KERNEL.borrow(cs).borrow_mut();
        let current = kernel.current_task;

        let wakeup_time = kernel.system_ticks + ticks;

        kernel.tasks[current].wakeup_time = wakeup_time;
        kernel.tasks[current].state = TaskState::Sleeping;

        kernel.ready.remove(current);
        kernel.sleeping.add(current);
    });

    scheduler::yield_now(); // immidietaly switch task
}

#[inline]
pub fn yield_cpu() {
    scheduler::yield_now();
}

pub unsafe fn start(syst: &mut cortex_m::peripheral::SYST, freq: u32) -> ! { // todo: disable interrupts here?
    scheduler::init_idle_task();
    scheduler::config_systick(syst, freq);

    unsafe {
        core::arch::asm!("svc 0", options(noreturn));
    }
}

pub fn exit() -> ! {
    critical_section::with(|cs| {
        let mut kernel = KERNEL.borrow(cs).borrow_mut();
        let current = kernel.current_task;

        kernel.ready.remove(current);
        kernel.sleeping.remove(current); // just in case, it should be impossible

        kernel.zombies.add(current);
        kernel.tasks[current].state = TaskState::Zombie;
    });

    scheduler::yield_now();

    loop { cortex_m::asm::wfi(); }
}

pub fn suspend(tid: TaskId) -> WhyResult<()> {
    let tid = tid.0;

    if tid == IDLE_TID {
        return Err(WhyError::InvalidOperation);
    }

    let should_yield = critical_section::with(|cs| {
        let mut kernel = KERNEL.borrow(cs).borrow_mut();

        let current = kernel.current_task;

        if !kernel.allocated.is_set(tid) {
            return Err(WhyError::InvalidTaskId);
        }

        let task = &mut kernel.tasks[tid];

        // already suspended
        if let TaskState::Suspended(_) = task.state {
            return Ok(false);
        }

        let ctx: ResumeContext = task.state.try_into()?;
        task.state = TaskState::Suspended(ctx);

        kernel.ready.remove(tid);
        kernel.sleeping.remove(tid);

        Ok(tid == current)
    })?;

    if should_yield {
        scheduler::yield_now();
    }
    Ok(())
}

pub fn resume(tid: TaskId) -> WhyResult<()> {
    let tid = tid.0;

    if tid == IDLE_TID {
        return Err(WhyError::InvalidOperation);
    }

    let should_yield = critical_section::with(|cs| {
        let mut kernel = KERNEL.borrow(cs).borrow_mut();

        if !kernel.allocated.is_set(tid) {
            return Err(WhyError::InvalidTaskId);
        }

        let now = kernel.system_ticks;
        let task = &mut kernel.tasks[tid];

        let TaskState::Suspended(ctx) = task.state else {
            return Ok(false);
        };

        //task.state = ctx.into(); // ResumeContext::Ready -> TaskState::Ready etc.

        match ctx {
            ResumeContext::Ready => {
                task.state = TaskState::Ready;
                kernel.ready.add(tid);
                Ok(true)
            },

            ResumeContext::Sleeping => {
                if task.wakeup_time <= now { // sleep expired while suspended
                    task.state = TaskState::Ready;
                    kernel.ready.add(tid);
                    Ok(true)
                } else {
                    task.state = TaskState::Sleeping;
                    kernel.sleeping.add(tid);
                    Ok(false) // didn't wake up yet
                }
            },

            ResumeContext::Blocked => {
                task.state = TaskState::Ready;
                kernel.ready.add(tid);
                Ok(true) // mutex will handle it
            },
        }
    })?;

    if should_yield {
        scheduler::yield_now();
    }

    Ok(())
}

#[inline]
pub fn current_tid() -> TaskId {
    critical_section::with(|cs| {
        let kernel = KERNEL.borrow(cs).borrow();
        TaskId(kernel.current_task)
    })
}

#[inline]
pub fn current_name() -> Option<&'static str> {
    critical_section::with(|cs| {
        let kernel = KERNEL.borrow(cs).borrow();
        kernel.tasks[kernel.current_task].name
    })
}

#[inline]
pub fn uptime_ticks() -> u64 {
    critical_section::with(|cs| {
        let kernel = KERNEL.borrow(cs).borrow();
        kernel.system_ticks
    })
}

#[inline]
pub fn task_count() -> usize {
    critical_section::with(|cs| {
        let kernel = KERNEL.borrow(cs).borrow();
        kernel.allocated.ones()
    })
}

#[inline]
pub fn reclaim_memory() {
    scheduler::reap_zombies();
}

#[inline]
pub fn task_info(tid: TaskId) -> WhyResult<TaskInfo> {
    critical_section::with(|cs| {
        let kernel = KERNEL.borrow(cs).borrow();
        let tid = tid.0;

        if !kernel.allocated.is_set(tid) {
            return Err(WhyError::InvalidTaskId);
        }

        let task = &kernel.tasks[tid];

        Ok(TaskInfo {
            id: tid,
            name: task.name,
            state: task.state,
            priority: task.priority,
            stack_size: task.stack_size,
        })
    })
}

pub fn active_tasks() -> impl Iterator<Item = TaskId> {
    let mask = critical_section::with(|cs| {
        let kernel = KERNEL.borrow(cs).borrow();
        kernel.allocated // copy it out
    });

    mask.iter().map(TaskId)
}