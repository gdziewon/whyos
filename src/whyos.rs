mod scheduler;
mod task;
mod itc;

pub use task::Stack;
pub use itc::Mutex;
use task::{TaskEntryPoint, Tcb, TaskState};

use crate::whyos::scheduler::{KERNEL, MAX_TASKS, config_systick, init_idle_task};

// fixme: very bad api, stack shouldnt need to be provided
pub fn add_task<const N: usize>(stack: &'static Stack<N>, entry: TaskEntryPoint, priority: u8) {
    let sp = stack.init(entry);

    critical_section::with(|cs| {
        let mut kernel = KERNEL.borrow(cs).borrow_mut();

        if kernel.task_count >= MAX_TASKS {
            panic!("Kernel Full: Max tasks reached!");
        }

        let idx = kernel.task_count;
        kernel.tasks[idx] = Tcb { sp, state: TaskState::Ready, priority, wakeup_time: 0 };
        kernel.task_count += 1;
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
