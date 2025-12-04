
use crate::memory;
use crate::task::{self, EXC_RETURN_THREAD_PSP, Tcb, TaskState, TaskList};

use core::ptr;
use core::{arch::naked_asm, cell::RefCell};
use cortex_m::peripheral::SCB;
use cortex_m_rt::exception;
use critical_section::Mutex;

pub const MAX_TASKS: usize = 32; // this should stay hardcoded
const IDLE_TID: usize = 0;
const IDLE_STACK_SIZE: usize = 4096; // todo: might be too much

pub struct KernelState {
    pub tasks: [Tcb; MAX_TASKS],
    pub current_task: usize,
    pub system_ticks: u64,

    pub allocated: TaskList, // who exists
    pub ready: TaskList, // wants CPU
    pub sleeping: TaskList, // waiting for time
    pub zombies: TaskList
}

pub static KERNEL: Mutex<RefCell<KernelState>> = Mutex::new(RefCell::new(KernelState {
    tasks: [Tcb::dead(); MAX_TASKS],
    current_task: IDLE_TID,
    system_ticks: 0,
    allocated: TaskList::from(1 << IDLE_TID), // first spot reserved for idle task
    ready: TaskList::from(1 << IDLE_TID),
    sleeping: TaskList::new(),
    zombies: TaskList::new()
}));

pub fn reap_zombies() -> bool {
    let mut reaped = false;

    critical_section::with(|cs| {
        let mut kernel = KERNEL.borrow(cs).borrow_mut();

        for tid in kernel.zombies.iter() {
            let task = &mut kernel.tasks[tid];

            let ptr = task.stack_base as *mut u8;
            let size = task.stack_size;

            if ptr != ptr::null_mut() && size > 0 {
                unsafe { memory::dealloc(ptr, size); }
            }

            kernel.zombies.remove(tid);
            kernel.allocated.remove(tid);

            kernel.tasks[tid] = Tcb::dead();

            reaped = true;
        }
    });

    reaped
}

pub fn init_idle_task() { // idle task is ran when every other task can't
    extern "C" fn idle_task() -> ! {
        loop {
            reap_zombies();
            cortex_m::asm::wfi();
        }
    }

    let stack = match memory::alloc(IDLE_STACK_SIZE) {
        Some(mem) => mem,
        None => panic!("WhyOS: Out of Memory")
    };

    let sp = unsafe { task::init_stack(stack.ptr, stack.size, idle_task)};

    critical_section::with(|cs| {
        let mut kernel = KERNEL.borrow(cs).borrow_mut();
        kernel.tasks[IDLE_TID] = Tcb::ready(sp, u8::MAX, stack.ptr as usize, stack.size);
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

        kernel.ready.remove(current);
    });
}

pub fn wake_task(tid: usize) {
    critical_section::with(|cs| {
        let mut kernel = KERNEL.borrow(cs).borrow_mut();

        kernel.tasks[tid].state = TaskState::Ready;

        kernel.ready.add(tid);
        kernel.sleeping.remove(tid);
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
extern "C" fn get_idle_task_sp() -> usize {
    critical_section::with(|cs| {
        let kernel = KERNEL.borrow(cs).borrow();
        kernel.tasks[IDLE_TID].sp
    })
}

#[unsafe(no_mangle)]
extern "C" fn switch_task(old_sp: usize) -> usize {
    critical_section::with(|cs| {
        let mut kernel = KERNEL.borrow(cs).borrow_mut();

        // save old sp
        let current = kernel.current_task;
        kernel.tasks[current].sp = old_sp;
        if kernel.tasks[current].state == TaskState::Running {
            kernel.tasks[current].state = TaskState::Ready;
        }

        // if ready task isn't found, fallback to idle task
        let mut best_task = IDLE_TID;
        let mut best_prio = u8::MAX;

        for tid in kernel.ready.iter() {
            let prio = kernel.tasks[tid].priority;

            if prio <= best_prio {
                best_prio = prio;
                best_task = tid;
            }
        }

        kernel.current_task = best_task;
        kernel.tasks[best_task].state = TaskState::Running;
        kernel.tasks[best_task].sp
    })
}

#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn PendSV() { // todo: implement FPU!!!!
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

        for tid in kernel.sleeping.iter() {
            let task = &mut kernel.tasks[tid];

            if task.wakeup_time <= now {
                task.state = TaskState::Ready;

                kernel.sleeping.remove(tid);
                kernel.ready.add(tid);
            }
        }
    });
    SCB::set_pendsv(); // handle switch in PendSV
}