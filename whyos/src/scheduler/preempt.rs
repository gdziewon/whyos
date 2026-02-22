use core::arch::naked_asm;
use cortex_m::peripheral::SCB;
use cortex_m_rt::exception;

use crate::TaskState;
use super::{KERNEL, IDLE_TID, MAX_TASKS};


#[unsafe(no_mangle)]
extern "C" fn get_idle_task_sp() -> usize {
    critical_section::with(|cs| {
        let kernel = KERNEL.borrow(cs).borrow();

        // TODO: make absolute sure its safe here
        unsafe { kernel.tasks[IDLE_TID].stack.as_ref().unwrap_unchecked().sp() }
    })
}

#[unsafe(no_mangle)]
extern "C" fn switch_task(old_sp: usize) -> usize {
    critical_section::with(|cs| {
        let mut kernel = KERNEL.borrow(cs).borrow_mut();
        let current = kernel.current_task;

        if let Some(stack) = kernel.tasks[current].stack.as_mut() {
            if !stack.check_canary() {
                panic!("KERNEL PANIC: Stack Overflow detected in Task {}", current.id());
            }

            stack.set_sp(old_sp);
        }

        // to not overwrite Blocked etc
        if kernel.tasks[current].state == TaskState::Running {
            kernel.tasks[current].state = TaskState::Ready;
        }

        // start searching from (current + 1) for round robin
        let next = (current.id() + 1) % MAX_TASKS;

        let best_task = kernel.ready
            .iter_from(next)
            .min_by_key(|&tid| kernel.tasks[tid].priority)
            .unwrap_or(IDLE_TID); // fallback to IDLE

        kernel.current_task = best_task;
        kernel.tasks[best_task].state = TaskState::Running;

        // TODO: MAKE ABSOLUTE SURE IF THIS IS SAFE
        // Should be safe, task in ready array mustn't be dead
        unsafe {
            kernel.tasks[best_task].stack.as_ref().unwrap_unchecked().sp()
        }
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