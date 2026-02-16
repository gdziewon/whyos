
use crate::{TaskId, task::{ResumeContext, STACK_CANARY, TaskMap, TaskState, TaskTable}};

use core::{arch::naked_asm, cell::RefCell};
use cortex_m::peripheral::SCB;
use cortex_m_rt::exception;
use critical_section::Mutex;
use defmt::warn;

pub type TaskMask = u32; // TODO: just an idea for now, should be used in bitmaps
pub const MAX_TASKS: usize = TaskMask::BITS as usize;
pub const IDLE_TID: TaskId = unsafe { TaskId::new_unchecked(0) };

pub struct KernelState {
    pub tasks: TaskTable,
    pub current_task: TaskId,
    pub system_ticks: u64,

    pub allocated: TaskMap, // who exists
    pub ready: TaskMap, // wants CPU
    pub sleeping: TaskMap, // waiting for time
    pub zombies: TaskMap // waiting to die
}

pub static KERNEL: Mutex<RefCell<KernelState>> = Mutex::new(RefCell::new(KernelState {
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
        let current = kernel.current_task;

        if kernel.tasks[current].state != TaskState::Dead {
            let canary_val = unsafe { *(kernel.tasks[current].stack_base as *const u32)};
            if canary_val != STACK_CANARY {
                panic!("KERNEL PANIC: Stack Overflow detected in Task {}", current.id());
            }
        }

        kernel.tasks[current].sp = old_sp;
        if kernel.tasks[current].state == TaskState::Running {
            kernel.tasks[current].state = TaskState::Ready;
        }

        // start searching from (current + 1) for round robin
        let next = (current.id() + 1) & (MAX_TASKS - 1); // bitwise and instead of modulo, MAX_TASKS is a power of two

        let best_task = kernel.ready
            .iter_from(next)
            .min_by_key(|&tid| kernel.tasks[tid].priority)
            .unwrap_or(IDLE_TID); // fallback to IDLE

        kernel.current_task = best_task;
        kernel.tasks[best_task].state = TaskState::Running;
        kernel.tasks[best_task].sp
    })
}

#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn PendSV() {
    naked_asm!(
        // tells the assembler that we are using fpu
        ".fpu fpv5-sp-d16",

        // load OLD sp to r0
        "mrs r0, psp",
        "isb",      // sync, mostly deffensive here

        // check if OLD task is using FPU
        // '!' updates r0
        "tst lr, #0x10",             // test bit 4 (identifies FPU usage)
        "it eq",                     // if FPU is enabled... (bit 4 == 0)
        "vstmdbeq r0!, {{s16-s31}}", // save s16-s31, update r0 (OLD sp)

        // push OLD tasks regs r4-r11 + lr, update r0 (OLD sp)
        "stmdb r0!, {{r4-r11, lr}}",

        // call to 'switch_task'
        // input: r0 holds OLD task's stack ptr
        // output: r0 will hold NEW task's stack ptr
        "bl switch_task",

        // restore regs from NEW task's stack (ptr in r0)
        "ldmia r0!, {{r4-r11, lr}}",

        // check if NEW task is using FPU
        // we check the RESTORED LR value
        "tst lr, #0x10",             // test bit 4 (identifies FPU usage)
        "it eq",                     // if FPU is enabled... (bit 4 == 0)
        "vldmiaeq r0!, {{s16-s31}}", // pop s16-s31, update r0 (NEW sp)

        // update actual sp to NEW task's one
        "msr psp, r0",
        "isb",      // pipeline flush, ensures CPU uses new stack ptr

        // exception return using NEW task's lr, so CPU knows wether to unstack FPU regs
        "bx lr",
    );
}


#[exception]
fn SysTick() {
    critical_section::with(|cs| {
        let mut kernel = KERNEL.borrow(cs).borrow_mut();

        kernel.system_ticks += 1;
        let now = kernel.system_ticks;

        // wake up sleeping tasks
        for tid in kernel.sleeping.iter() {
            let task = &mut kernel.tasks[tid];

            if task.wakeup_time <= now {
                task.state = TaskState::Ready;

                kernel.sleeping.remove(tid);
                kernel.ready.add(tid);
            }
        }

        // software watchdog monitoring - ONLY FOR READY TASKS
        for tid in kernel.ready.iter() {
            let task = &mut kernel.tasks[tid];

            if let Some(bowl) = task.watchdog_remaining_ticks.as_mut() {
                if *bowl == 0 {
                    panic!("Task {} ({}) didn't feed the watchdog for {}",
                        tid.id(), task.name.unwrap_or("'no name'"), task.watchdog_interval_ticks);
                }
                *bowl -= 1;
            }
        }
    });
    SCB::set_pendsv(); // handle switch in PendSV
}