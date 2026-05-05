use crate::scheduler::Kernel;
use crate::task::{self, TaskStack, TaskId};
use crate::error::{WhyError, WhyResult};
use crate::memory;

// fixme: do we need this file

pub fn spawn(
    entry: task::TaskEntryPoint,
    arg: usize,
    name: Option<&'static str>,
    priority: u8,
    stack_size: usize
) -> WhyResult<TaskId> {
    let mem = match memory::alloc(stack_size) {
        Some(mem) => mem,
        None => {
            reap_zombies();
            memory::alloc(stack_size).ok_or(WhyError::OutOfMemory)?
        }
    };

    let ret = super::task_exit_trampoline as *const () as usize;
    let stack = TaskStack::init(mem, entry, arg, ret);

    Kernel::lock(|k| {
        k.spawn_task(name, priority, stack)
    })
}

pub fn reap_zombies() -> usize {
    let mut reaped_size = 0;

    Kernel::lock(|k| {
        for tid in k.zombies().iter() {
            if let Some(stack) = k.remove_zombie(tid) {
                reaped_size += stack.size();
                // here stack goes out of scope MemChunk automatically should be cleared
            }
        }
    });

    reaped_size
}