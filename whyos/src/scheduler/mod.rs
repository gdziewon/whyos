mod preempt;
mod kernel;

use core::cell::RefCell;

use critical_section::Mutex;
use defmt::warn;
use kernel::Kernel;
pub use kernel::{MAX_TASKS, TaskMask, IDLE_TID};

use crate::{ResumeContext, TaskId, TaskState, task::{TaskMap, TaskTable}};

pub static KERNEL: Mutex<RefCell<Kernel>> = Mutex::new(RefCell::new(Kernel {
    tasks: TaskTable::new(),
    current_task: IDLE_TID,
    system_ticks: 0,
    allocated: TaskMap::from(1 << IDLE_TID.id()), // first spot reserved for idle task
    ready: TaskMap::from(1 << IDLE_TID.id()),
    sleeping: TaskMap::new(),
    zombies: TaskMap::new()
}));

pub fn config_systick(syst: &mut cortex_m::peripheral::SYST, freq: u32) {
    syst.set_clock_source(cortex_m::peripheral::syst::SystClkSource::Core);
    syst.set_reload(freq);
    syst.clear_current();
    syst.enable_counter();
    syst.enable_interrupt();
}

pub fn block_current_task() {
    critical_section::with(|cs| {
        let mut kernel = KERNEL.borrow(cs).borrow_mut();

        let curr = kernel.current_task;
        kernel.tasks[curr].state = TaskState::Blocked;

        kernel.ready.remove(curr);
    });
}

pub fn wake_task(tid: TaskId) {
    critical_section::with(|cs| {
        let mut kernel = KERNEL.borrow(cs).borrow_mut();
        let task = &mut kernel.tasks[tid];

        match task.state {
            TaskState::Blocked => {
                task.state = TaskState::Ready;
                kernel.ready.add(tid);
                kernel.sleeping.remove(tid);
            },
            TaskState::Suspended(ResumeContext::Blocked) => {
                task.state = TaskState::Suspended(ResumeContext::Ready);
            },
            _ => { warn!("WhyOS: Waking non blocked task, should never happen")}
        }

    });
}

pub fn get_task_priority(tid: TaskId) -> u8 {
    critical_section::with(|cs| {
        let kernel = KERNEL.borrow(cs).borrow();
        kernel.tasks[tid].priority
    })
}

pub fn get_current_tid() -> TaskId {
    critical_section::with(|cs| {
        KERNEL.borrow(cs).borrow().current_task
    })
}

pub fn is_task_suspended(tid: TaskId) -> bool {
    critical_section::with(|cs| {
        let kernel = KERNEL.borrow(cs).borrow();
        matches!(kernel.tasks[tid].state, TaskState::Suspended(_))
    })
}

#[inline]
pub fn yield_now() {
    cortex_m::peripheral::SCB::set_pendsv();
}
