#![no_std]

mod scheduler;
mod task;
mod itc;
mod memory;
mod error;

use defmt::error;
pub use itc::{Mutex, Queue, Semaphore};
pub use task::TaskId;

use task::{TaskEntryPoint, Tcb, TaskState, ResumeContext};
use error::{WhyError, WhyResult};
use scheduler::{KERNEL, MAX_TASKS, config_systick, init_idle_task, IDLE_TID};

pub fn add_task(entry: TaskEntryPoint, priority: u8, stack_size: usize) -> WhyResult<TaskId> {
    let stack = match memory::alloc(stack_size) {
        Some(mem) => mem,
        None => {
            scheduler::reap_zombies();
            memory::alloc(stack_size).ok_or(WhyError::OutOfMemory)?
        }
    };

    let sp = unsafe { task::init_stack(stack.ptr, stack.size, entry)};

    critical_section::with(|cs| {
        let mut kernel = KERNEL.borrow(cs).borrow_mut();

        let tid = (!kernel.allocated.0).trailing_zeros() as usize;
        if tid >= MAX_TASKS {
            return Err(WhyError::MaxTasksReached);
        }

        kernel.allocated.add(tid);
        kernel.ready.add(tid);
        kernel.tasks[tid] = Tcb::ready(sp, priority, stack.ptr as usize, stack.size);
        Ok(TaskId(tid))
    })
}

pub fn sleep(ticks: u64) {
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

pub unsafe fn start(syst: &mut cortex_m::peripheral::SYST, freq: u32) -> ! {
    init_idle_task(); // todo: move it to KernelState initialization?
    config_systick(syst, freq);
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

    loop { cortex_m::asm::nop(); }
}

pub fn suspend(tid: TaskId) -> WhyResult<()> {
    critical_section::with(|cs| {
        let mut kernel = KERNEL.borrow(cs).borrow_mut();

        let tid = tid.0;
        if tid == IDLE_TID || !kernel.allocated.is_set(tid) {
            error!("TID IS NOT SET {}", tid);
            return Err(WhyError::InvalidOperation); // fixme: invalid tid
        }

        let task = &mut kernel.tasks[tid];

        if let TaskState::Suspended(_) = task.state {
            return Ok(());
        }

        let ctx: ResumeContext = task.state.try_into()?;
        task.state = TaskState::Suspended(ctx);

        kernel.ready.remove(tid);
        kernel.sleeping.remove(tid);

        Ok(())
    })?;

    scheduler::yield_now();
    Ok(())
}

// todo: make some TID struct for correctness?
pub fn resume(tid: TaskId) -> WhyResult<()> {
    let mut woken = false;

    critical_section::with(|cs| {
        let mut kernel = KERNEL.borrow(cs).borrow_mut();

        let tid = tid.0;
        if tid == IDLE_TID || !kernel.allocated.is_set(tid) {
            return Err(WhyError::InvalidOperation); // fixme: invalid tid - TID struct?
        }

        let now = kernel.system_ticks;
        let task = &mut kernel.tasks[tid];

        if let TaskState::Suspended(ctx) = task.state {
            task.state = ctx.into(); // ResumeContext::Ready -> TaskState::Ready etc.
            woken = true;

            match ctx {
                ResumeContext::Ready => kernel.ready.add(tid),

                ResumeContext::Sleeping => {
                    if task.wakeup_time <= now { // edge case for previously sleeping tasks
                        task.state = TaskState::Ready;
                        kernel.ready.add(tid);

                    } else {
                        kernel.sleeping.add(tid);
                        woken = false; // didn't actually wake up
                    }
                },

                ResumeContext::Blocked => {}, // Mutex will handle it
            }
        }

        Ok(())
    })?;

    if woken {
        scheduler::yield_now();
    }
    Ok(())
}