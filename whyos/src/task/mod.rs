mod builder;
mod stack;
mod registry;
mod types;

pub use builder::{TaskBuilder, TaskRoutine, TaskRoutineArg, StackSize};
pub use stack::{Stack, TaskEntryPoint, TaskStack};
pub use registry::{TaskRegistry, Tcb, TaskMap, Watchdog, TaskMask};
pub use types::{TaskId, TaskInfo, TaskState, ResumeContext, BlockReason, TaskHandle};

use crate::{error::{WhyResult, WhyError}, memory, scheduler::Kernel};

pub(crate) extern "C" fn task_exit_trampoline() -> ! {
	crate::exit()
}

pub fn spawn(
    entry: TaskEntryPoint,
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

    let ret = task_exit_trampoline as *const () as usize;
    let stack = TaskStack::init(mem, entry, arg, ret);

    Kernel::lock(|k| {
        k.spawn_task(name, priority, stack)
    })
}