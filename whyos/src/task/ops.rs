use crate::scheduler::{self, KERNEL, IDLE_TID};
use crate::task::{self, TaskId, TaskState, Tcb};
use crate::error::{WhyError, WhyResult};
use crate::memory;

const IDLE_STACK_SIZE: usize = 4096; // todo: might be too much

extern "C" fn task_exit_trampoline() -> ! {
    remove_task();
    panic!()
}

pub fn spawn(
    entry: task::TaskEntryPoint,
    arg: usize,
    name: Option<&'static str>,
    priority: u8,
    stack_size: usize
) -> WhyResult<TaskId> {
    let stack = match memory::alloc(stack_size) {
        Some(mem) => mem,
        None => {
            reap_zombies();
            memory::alloc(stack_size).ok_or(WhyError::OutOfMemory)?
        }
    };

    let sp = unsafe {
        task::init_stack(
            &stack,
            entry,
            arg,
            task_exit_trampoline as *const () as usize
        )
    };

    critical_section::with(|cs| {
        let mut kernel = KERNEL.borrow(cs).borrow_mut();

        // FIXME: do sth about it in next commit
        let tid = TaskId::new((!kernel.allocated.0).trailing_zeros() as usize)?;

        kernel.allocated.add(tid);
        kernel.ready.add(tid);
        kernel.tasks[tid] = Tcb::ready(name, sp, priority, stack);
        Ok(tid)
    })
}

pub fn remove_task() {
    critical_section::with(|cs| {
        let mut kernel = KERNEL.borrow(cs).borrow_mut();
        let current = kernel.current_task;

        kernel.ready.remove(current);
        kernel.sleeping.remove(current); // just in case, it should be impossible

        kernel.zombies.add(current);
        kernel.tasks[current].state = TaskState::Zombie;
    });

    scheduler::yield_now();
}

pub fn reap_zombies() -> usize {
    let mut reaped_size = 0;

    critical_section::with(|cs| {
        let mut kernel = KERNEL.borrow(cs).borrow_mut();

        for tid in kernel.zombies.iter() {
            let task = &mut kernel.tasks[tid];

            if let Some(stack) = task.stack.take() {
                reaped_size += stack.size();

                kernel.zombies.remove(tid);
                kernel.allocated.remove(tid);

                kernel.tasks[tid] = Tcb::dead();

                unsafe { memory::dealloc(stack); }
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

    let stack = match memory::alloc(IDLE_STACK_SIZE) {
        Some(mem) => mem,
        None => panic!("WhyOS: Couldn't allocate Idle Task")
    };


    let return_handler = task_exit_trampoline as *const () as usize;
    let sp = unsafe {
        task::init_stack(
            &stack,
            idle_task,
            0,
            return_handler
        )
    };

    critical_section::with(|cs| {
        let mut kernel = KERNEL.borrow(cs).borrow_mut();
        kernel.tasks[IDLE_TID] = Tcb::ready(Some("idle"), sp, u8::MAX, stack);
    });
}