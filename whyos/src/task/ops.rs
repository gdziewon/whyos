use crate::scheduler::{self, Kernel, IDLE_TID};
use crate::task::{self, Stack, TaskId};
use crate::error::{WhyError, WhyResult};
use crate::memory;

const IDLE_STACK_SIZE: usize = 1024;

extern "C" fn task_exit_trampoline() -> ! {
    kill_current_task();
    panic!()
}

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

    let ret = task_exit_trampoline as *const () as usize;
    let stack = Stack::init(mem, entry, arg, ret);

    Kernel::lock(|k| {
        k.spawn_task(name, priority, stack)
    })
}

pub fn kill_current_task() {
    Kernel::lock(|k| {
        k.make_zombie(k.current_task());
    });

    scheduler::yield_now();
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

pub fn init_idle_task() { // idle task is ran when every other task can't
    extern "C" fn idle_task(_: usize) {
        loop {
            reap_zombies();
            cortex_m::asm::wfi();
        }
    }

    let mem = match memory::alloc(IDLE_STACK_SIZE) {
        Some(mem) => mem,
        None => panic!("WhyOS: Couldn't allocate Idle Task")
    };

    let return_handler = task_exit_trampoline as *const () as usize;
    let stack = Stack::init(mem, idle_task, 0, return_handler);

    Kernel::lock(|k| {
        k.init_task(IDLE_TID, Some("idle"), u8::MAX, stack);
    });
}