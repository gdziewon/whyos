#![no_std]

mod scheduler;
mod task;
mod itc;
mod memory;

pub use itc::{Mutex, Queue, Semaphore};
use task::{TaskEntryPoint, Tcb, TaskState};

use scheduler::{KERNEL, MAX_TASKS, config_systick, init_idle_task};

// fixme: very bad api, stack shouldnt need to be provided
pub fn add_task(entry: TaskEntryPoint, priority: u8, stack_size: usize) {
    let stack = match memory::alloc(stack_size) {
        Some(mem) => mem,
        None => panic!("WhyOS: Out of Memory")
    };

    let sp = unsafe { task::init_stack(stack.ptr, stack.size, entry)};

    critical_section::with(|cs| {
        let mut kernel = KERNEL.borrow(cs).borrow_mut();

        let free_tid = (!kernel.allocated.0).trailing_zeros() as usize;
        if free_tid >= MAX_TASKS {
            panic!("WhyOS: Max tasks reached");
        }

        kernel.allocated.add(free_tid);
        kernel.tasks[free_tid] = Tcb::new(sp, priority, stack.ptr as usize, stack.size);
    });
}

pub fn sleep(ticks: u64) {
    critical_section::with(|cs| {
        let mut kernel = KERNEL.borrow(cs).borrow_mut();
        let current = kernel.current_task;

        let wakeup_time = kernel.system_ticks + ticks;

        kernel.tasks[current].wakeup_time = wakeup_time;
        kernel.tasks[current].state = TaskState::Blocked;
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
