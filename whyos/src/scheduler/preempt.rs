use core::arch::naked_asm;
use cortex_m::peripheral::SCB;
use cortex_m_rt::exception;

use crate::scheduler::Kernel;

#[unsafe(no_mangle)]
extern "C" fn get_idle_task_sp() -> usize {
    Kernel::lock(|k| k.idle_sp())
}

#[unsafe(no_mangle)]
extern "C" fn switch_task(old_sp: usize) -> usize {
    Kernel::lock(|k| k.schedule(old_sp))
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
    Kernel::lock(|k| {
        let now = k.tick();

        // wake up sleeping tasks
        for tid in k.sleeping().iter() {
            let task = k.task(tid);
            if task.wakeup_time <= now {
                k.wake_task(tid);
            }
        }

        // software watchdog monitoring - ONLY FOR READY TASKS
        for tid in k.ready().iter() {
            k.watchdog_check(tid);
        }
    });

    SCB::set_pendsv(); // handle switch in PendSV
}