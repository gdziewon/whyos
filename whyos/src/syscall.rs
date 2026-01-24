use crate::scheduler::{self, KERNEL, IDLE_TID};
use crate::task::{TaskId, TaskState, ResumeContext, TaskInfo};
use crate::error::{WhyError, WhyResult};

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

pub fn current_tid() -> TaskId {
    critical_section::with(|cs| {
        let kernel = KERNEL.borrow(cs).borrow();
        TaskId(kernel.current_task)
    })
}

pub fn current_name() -> Option<&'static str> {
    critical_section::with(|cs| {
        let kernel = KERNEL.borrow(cs).borrow();
        kernel.tasks[kernel.current_task].name
    })
}

pub fn uptime_ticks() -> u64 {
    critical_section::with(|cs| {
        KERNEL.borrow(cs).borrow().system_ticks
    })
}

pub fn task_count() -> usize {
    critical_section::with(|cs| {
        KERNEL.borrow(cs).borrow().allocated.ones()
    })
}

pub fn active_tasks() -> impl Iterator<Item = TaskId> {
    let mask = critical_section::with(|cs| {
        let kernel = KERNEL.borrow(cs).borrow();
        kernel.allocated // copy it out
    });

    mask.iter().map(TaskId)
}

pub fn yield_now() {
    scheduler::yield_now();
}