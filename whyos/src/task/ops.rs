use crate::scheduler::Kernel;
use crate::task::{self, TaskHandle, TaskStack};
use crate::error::{WhyError, WhyResult};
use crate::memory;

// fixme: do we need this file

pub fn spawn(
    entry: task::TaskEntryPoint,
    arg: usize,
    name: Option<&'static str>,
    priority: u8,
    stack_size: usize
) -> WhyResult<TaskHandle> {
    let mem = match memory::alloc(stack_size) {
        Some(mem) => mem,
        None => {
            Kernel::lock(|k| k.reap_zombies());
            memory::alloc(stack_size).ok_or(WhyError::OutOfMemory)?
        }
    };

    let ret = super::task_exit_trampoline as *const () as usize;
    let stack = TaskStack::init(mem, entry, arg, ret);

    Kernel::lock(|k| {
        k.spawn_task(name, priority, stack)
    })
}

