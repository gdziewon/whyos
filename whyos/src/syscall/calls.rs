use crate::scheduler::{self, KERNEL, IDLE_TID};
use crate::task::{ResumeContext, TaskId, TaskInfo, TaskMap, TaskState, ops};
use crate::error::{WhyError, WhyResult};

#[inline]
pub fn reboot() -> ! {
    cortex_m::peripheral::SCB::sys_reset();
}

#[inline]
pub fn yield_now() {
    scheduler::yield_now();
}

#[inline]
pub fn exit() {
    ops::remove_task()
}

#[inline]
pub fn reclaim_memory() -> usize {
    ops::reap_zombies()
}

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

pub fn suspend(tid: TaskId) -> WhyResult<()> {
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
        kernel.sleeping.remove(tid); // todo: for sure?

        Ok(tid == current)
    })?;

    if should_yield {
        scheduler::yield_now();
    }
    Ok(())
}

pub fn resume(tid: TaskId) -> WhyResult<()> {
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

        match ctx {
            ResumeContext::Ready | ResumeContext::Blocked => {
                task.state = TaskState::Ready;
                kernel.ready.add(tid);
                Ok(true)
            },

            ResumeContext::Sleeping => {
                if task.wakeup_time <= now { // sleep expired while suspended
                    task.state = TaskState::Ready;
                    kernel.ready.add(tid);
                    Ok(true)
                } else { // didn't wake up yet
                    task.state = TaskState::Sleeping;
                    kernel.sleeping.add(tid);
                    Ok(false)
                }
            },
        }
    })?;

    if should_yield {
        scheduler::yield_now();
    }

    Ok(())
}

pub fn get_task_info(tid: TaskId) -> WhyResult<TaskInfo> {
    critical_section::with(|cs| {
        let kernel = KERNEL.borrow(cs).borrow();

        if !kernel.allocated.is_set(tid) {
            return Err(WhyError::InvalidTaskId);
        }

        let task = &kernel.tasks[tid];

        if let Some(stack) = &task.stack {
            Ok(TaskInfo {
                tid,
                name: task.name,
                state: task.state,
                priority: task.priority,
                current_sp: stack.sp(),
                stack_base: stack.base() as usize,
                stack_size: stack.size(),
                max_stack_usage: stack.usage(),
            })
        } else {
            Err(WhyError::InternalError) // FIXME: add more errors
        }
    })
}

pub fn get_current_tid() -> TaskId {
    critical_section::with(|cs| {
        let kernel = KERNEL.borrow(cs).borrow();
        kernel.current_task
    })
}

pub fn get_current_name() -> Option<&'static str> {
    critical_section::with(|cs| {
        let kernel = KERNEL.borrow(cs).borrow();
        kernel.tasks[kernel.current_task].name
    })
}

pub fn get_uptime_ticks() -> u64 {
    critical_section::with(|cs| {
        KERNEL.borrow(cs).borrow().system_ticks
    })
}

pub fn get_task_count() -> usize {
    critical_section::with(|cs| {
        KERNEL.borrow(cs).borrow().allocated.ones()
    })
}

pub fn get_allocated_tasks() -> TaskMap {
    critical_section::with(|cs| {
        let kernel = KERNEL.borrow(cs).borrow();
        kernel.allocated // copy it out
    })
}

pub fn watchdog_subscribe(interval_ticks: u64) {
    if interval_ticks == 0 {
        return;
    }

    critical_section::with(|cs| {
        let mut kernel = KERNEL.borrow(cs).borrow_mut();
        let tid = kernel.current_task;
        let task = &mut kernel.tasks[tid];

        task.watchdog_interval_ticks = interval_ticks;
        task.watchdog_remaining_ticks = Some(interval_ticks);
    })
}

pub fn watchdog_unsubscribe() {
    critical_section::with(|cs| {
        let mut kernel = KERNEL.borrow(cs).borrow_mut();
        let tid = kernel.current_task;
        let task = &mut kernel.tasks[tid];

        task.watchdog_interval_ticks = 0;
        task.watchdog_remaining_ticks = None;
    })
}

pub fn watchdog_feed() {
    critical_section::with(|cs| {
        let mut kernel = KERNEL.borrow(cs).borrow_mut();
        let tid = kernel.current_task;
        let task = &mut kernel.tasks[tid];

        if let Some(bowl) = task.watchdog_remaining_ticks.as_mut() {
            *bowl = task.watchdog_interval_ticks;
        }
    })
}