mod stack;

pub use stack::Stack;

use core::{arch::naked_asm, cell::RefCell};
use cortex_m::peripheral::SCB;
use cortex_m_rt::exception;
use critical_section::Mutex;

const MAX_TASKS: usize = 16;
pub const EXC_RETURN_THREAD_PSP: u32 = 0xFFFFFFFD;

pub type TaskEntryPoint = extern "C" fn() -> !;

// inspired by https://freertos.org/Documentation/02-Kernel/02-Kernel-features/01-Tasks-and-co-routines/02-Task-states
#[derive(Clone, Copy, PartialEq, Eq)]
enum TaskState {
    Ready,
    Running,
    Blocked,
    Suspended
}

#[derive(Clone, Copy)]
struct Tcb { // task control block
    pub sp: u32,
    pub state: TaskState,
    pub priority: u8, // lower number = higher priority
    pub wakeup_time: u64
}

struct KernelState {
    tasks: [Tcb; MAX_TASKS], // todo: make it MaybeUninit array?
    current_task: usize,
    task_count: usize,
    system_ticks: u64
}

// todo: maybe RefCell can be removed somehow
static KERNEL: Mutex<RefCell<KernelState>> = Mutex::new(RefCell::new(KernelState {
    tasks: [Tcb { sp: 0, state: TaskState::Ready, priority: u8::MAX, wakeup_time: 0 }; MAX_TASKS],
    current_task: 0,
    task_count: 1, // first spot reserved for idle task
    system_ticks: 0
}));

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
    cortex_m::peripheral::SCB::set_pendsv(); // immidietaly switch task
}

static IDLE_STACK: Stack<4096> = Stack::new();

fn init_idle_task() { // idle task is ran when every other task can't
    extern "C" fn idle_task() -> ! {
        loop {
            cortex_m::asm::wfi();
        }
    }

    let sp = IDLE_STACK.init(idle_task);

    critical_section::with(|cs| {
        let mut kernel = KERNEL.borrow(cs).borrow_mut();
        kernel.tasks[0] = Tcb { // idle task will have index 0 for simplicity
            sp,
            state: TaskState::Ready,
            priority: u8::MAX,
            wakeup_time: 0
        };
    });
}

fn config_systick(syst: &mut cortex_m::peripheral::SYST, freq: u32) {
    syst.set_clock_source(cortex_m::peripheral::syst::SystClkSource::Core);
    syst.set_reload(freq);
    syst.clear_current();
    syst.enable_counter();
    syst.enable_interrupt();
}

pub unsafe fn start(syst: &mut cortex_m::peripheral::SYST, freq: u32) -> ! {
    init_idle_task(); // todo: move it to KernelState initialization?
    config_systick(syst, freq);
    unsafe {
        core::arch::asm!("svc 0", options(noreturn));
    }
}

#[unsafe(no_mangle)]
extern "C" fn switch_task(old_sp: u32) -> u32 {
    critical_section::with(|cs| {
        let mut kernel = KERNEL.borrow(cs).borrow_mut();

        // save old sp
        let current = kernel.current_task;
        kernel.tasks[current].sp = old_sp;

        // if ready task isn't found, fallback to idle task
        let mut best_task = 0;
        let mut best_prio = u8::MAX;

        for i in 1..=kernel.task_count {
            let idx = (current + i) % kernel.task_count; // priority + round robin for prio ties

            // needed in case if user defines task with priority 255, idle task should never win ties in this case
            if idx == 0 { continue; } // todo: look for better design

            let task = &kernel.tasks[idx];

            if task.state == TaskState::Ready && task.priority <= best_prio {
                best_prio = task.priority;
                best_task = idx;
            }
        }

        kernel.current_task = best_task;
        kernel.tasks[best_task].sp
    })
}

#[unsafe(no_mangle)]
extern "C" fn get_first_task_sp() -> u32 {
    critical_section::with(|cs| {
        let kernel = KERNEL.borrow(cs).borrow();
        kernel.tasks[0].sp
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
        "bl get_first_task_sp",
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