
pub use super::task::Stack;
use super::task::{EXC_RETURN_THREAD_PSP, Tcb, TaskState};

use core::{arch::naked_asm, cell::RefCell};
use cortex_m::peripheral::SCB;
use cortex_m_rt::exception;
use critical_section::Mutex;

pub const MAX_TASKS: usize = 16;
static IDLE_STACK: Stack<4096> = Stack::new();
const IDLE_TID: usize = 0;

pub struct KernelState {
    pub tasks: [Tcb; MAX_TASKS], // todo: make it MaybeUninit array?
    pub current_task: usize,
    pub task_count: usize,
    pub system_ticks: u64
}

// todo: maybe RefCell can be removed somehow
pub static KERNEL: Mutex<RefCell<KernelState>> = Mutex::new(RefCell::new(KernelState {
    tasks: [Tcb { sp: 0, state: TaskState::Ready, priority: u8::MAX, wakeup_time: 0 }; MAX_TASKS],
    current_task: IDLE_TID,
    task_count: 1, // first spot reserved for idle task
    system_ticks: 0
}));

pub fn init_idle_task() { // idle task is ran when every other task can't
    extern "C" fn idle_task() -> ! {
        loop {
            cortex_m::asm::wfi();
        }
    }

    let sp = IDLE_STACK.init(idle_task);

    critical_section::with(|cs| {
        let mut kernel = KERNEL.borrow(cs).borrow_mut();
        kernel.tasks[IDLE_TID] = Tcb { // idle task will have index 0 for simplicity
            sp,
            state: TaskState::Ready,
            priority: u8::MAX,
            wakeup_time: 0
        };
    });
}

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
        let current = kernel.current_task;
        kernel.tasks[current].state = TaskState::Blocked;
    });
}

pub fn wake_task(tid: usize) {
    critical_section::with(|cs| {
        let mut kernel = KERNEL.borrow(cs).borrow_mut();
        kernel.tasks[tid].state = TaskState::Ready;
    });
}

pub fn get_task_priority(tid: usize) -> u8 {
    critical_section::with(|cs| {
        let kernel = KERNEL.borrow(cs).borrow();
        kernel.tasks[tid].priority
    })
}

pub fn get_current_tid() -> usize {
    critical_section::with(|cs| {
        KERNEL.borrow(cs).borrow().current_task
    })
}

#[inline]
pub fn yield_now() {
    cortex_m::peripheral::SCB::set_pendsv();
}

#[unsafe(no_mangle)]
extern "C" fn get_idle_task_sp() -> u32 {
    critical_section::with(|cs| {
        let kernel = KERNEL.borrow(cs).borrow();
        kernel.tasks[IDLE_TID].sp
    })
}

#[unsafe(no_mangle)]
extern "C" fn switch_task(old_sp: u32) -> u32 {
    critical_section::with(|cs| {
        let mut kernel = KERNEL.borrow(cs).borrow_mut();

        // save old sp
        let current = kernel.current_task;
        kernel.tasks[current].sp = old_sp;

        // if ready task isn't found, fallback to idle task
        let mut best_task = IDLE_TID;
        let mut best_prio = u8::MAX;

        for i in 1..=kernel.task_count {
            let tid = (current + i) % kernel.task_count; // priority + round robin for prio ties

            // needed in case if user defines task with priority 255, idle task should never win ties in this case
            if tid == IDLE_TID { continue; } // todo: look for better design

            let task = &kernel.tasks[tid];

            if task.state == TaskState::Ready && task.priority <= best_prio {
                best_prio = task.priority;
                best_task = tid;
            }
        }

        kernel.current_task = best_task;
        kernel.tasks[best_task].sp
    })
}

#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn PendSV() {
    naked_asm!(
        "mrs r0, psp",            // move psp to r0
        "isb",                    // sync barrier
        "stmdb r0!, {{r4-r11}}",  // push regs r4-r11 onto r0 (psp) and update it
        "push {{lr}}",            // push LR (tells the cpu what it was doing before it was interrupted)

        "bl switch_task",         // switch task - save old sp and get new one (into r0)

        "pop {{r1}}",             // pop LR value
        "mov lr, r1",             // move it to LR reg
        "ldmia r0!, {{r4-r11}}",  // pop saved regs of new task, update r0
        "msr psp, r0",            // set psp to r0
        "isb",                    // sync barrier
        "bx lr",                  // pop hw frame and run the task (thread mode, psp)
    );
}

#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn SVCall() {
    naked_asm!(
        "bl get_idle_task_sp",
        "ldmia r0!, {{r4-r11}}",// discard software frame (update r0 to point at hardware frame)

        "msr psp, r0",          // set psp to r0 (hw frame)

        "mov r0, {EXC_VAL}",
        "mov lr, r0",           // set lr to EXC_RETURN_THREAD_PSP

        "bx lr",                // pop hw frame and run the task (thread mode, psp)
        EXC_VAL = const EXC_RETURN_THREAD_PSP,
    );
}

#[exception]
fn SysTick() {
    critical_section::with(|cs| {
        let mut kernel = KERNEL.borrow(cs).borrow_mut();

        kernel.system_ticks += 1;
        let now = kernel.system_ticks;

        for i in 1..kernel.task_count { // skip idle
            let task = &mut kernel.tasks[i];

            // wake up sleeping task if its time
            if task.state == TaskState::Blocked && task.wakeup_time <= now {
                task.state = TaskState::Ready;
            }
        }
    });
    SCB::set_pendsv(); // handle switch in PendSV
}