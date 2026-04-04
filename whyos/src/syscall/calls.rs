use crate::scheduler::{self, Kernel};
use crate::task::{TaskId, TaskInfo, TaskMap, ops};
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
    ops::kill_current_task()
}

#[inline]
pub fn reclaim_memory() -> usize {
    ops::reap_zombies()
}

pub fn sleep(ticks: u64) {
    if ticks > 0 {
        Kernel::lock(|k| {
            let curr = k.current_task().expect("WhyOS: no current task");
            k.sleep_task(curr, ticks);
        });
    }

    scheduler::yield_now(); // immidietaly switch task
}

pub fn suspend(tid: TaskId) -> WhyResult<()> {
    let should_yield = Kernel::lock(|k| k.suspend_task(tid))?;

    if should_yield {
        scheduler::yield_now();
    }
    Ok(())
}

pub fn resume(tid: TaskId) -> WhyResult<()> {
    let should_yield = Kernel::lock(|k| k.resume_task(tid))?;

    if should_yield {
        scheduler::yield_now();
    }

    Ok(())
}

pub fn get_task_info(tid: TaskId) -> WhyResult<TaskInfo> {

    Kernel::lock(|k| {
        if !k.allocated().is_set(tid) {
            return Err(WhyError::InvalidTaskId);
        }

        let task = k.task(tid);

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
    Kernel::lock(|k| k.current_task().expect("WhyOS: no current task"))
}

pub fn get_current_name() -> Option<&'static str> {
    Kernel::lock(|k| {
        let curr = k.current_task().expect("WhyOS: no current task");
        k.task(curr).name
    })
}

pub fn get_uptime_ticks() -> u64 {
    Kernel::lock(|k| k.system_ticks())
}

pub fn get_task_count() -> usize {
    Kernel::lock(|k| k.allocated().ones())
}

pub fn get_allocated_tasks() -> TaskMap {
    Kernel::lock(|k| k.allocated())
}

pub fn watchdog_subscribe(interval_ticks: u64) {
    if interval_ticks == 0 {
        return;
    }

    Kernel::lock(|k| {
        let curr = k.current_task().expect("WhyOS: no current task");
        k.watchdog_subscribe(curr, interval_ticks);
    })
}

pub fn watchdog_unsubscribe() {
    Kernel::lock(|k| {
        let curr = k.current_task().expect("WhyOS: no current task");
        k.watchdog_unsubscribe(curr);
    })
}

pub fn watchdog_feed() {
    Kernel::lock(|k| {
        let curr = k.current_task().expect("WhyOS: no current task");
        k.watchdog_feed(curr);
    })
}