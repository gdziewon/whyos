use core::arch::naked_asm;
use cortex_m::peripheral::SCB;
use cortex_m_rt::exception;

use crate::scheduler::Kernel;

#[unsafe(no_mangle)]
extern "C" fn switch_task(old_sp: usize) -> usize {
    Kernel::lock(|k| k.schedule(old_sp))
}

#[cfg(target_abi = "eabihf")] // with fpu
#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn PendSV() {
    naked_asm!(
        ".fpu fpv5-sp-d16", // tells the assembler that we are using fpu

        // load OLD sp to r0
        "mrs r0, psp",

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

        "msr psp, r0", // update actual sp to NEW task's one

        // exception return using NEW task's lr, so CPU knows wether to unstack FPU regs
        "bx lr",
    );
}


#[cfg(not(target_abi = "eabihf"))] // no fpu
#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn PendSV() {
    naked_asm!(
        // load OLD sp to r0
        "mrs r0, psp",

        // push OLD tasks regs r4-r11 + lr, update r0 (OLD sp)
        "stmdb r0!, {{r4-r11, lr}}",

        // call to 'switch_task'
        // input: r0 holds OLD task's stack ptr
        // output: r0 will hold NEW task's stack ptr
        "bl switch_task",

        // restore regs from NEW task's stack (ptr in r0)
        "ldmia r0!, {{r4-r11, lr}}",

        "msr psp, r0", // update actual sp to NEW task's one

        // exception return using NEW task's lr, so CPU knows wether to unstack FPU regs
        "bx lr",
    );
}

#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn SVCall() {
    naked_asm!(
        // setup PSP
        "bl get_idle_task_sp",       // returns idle sp in r0
        "ldmia r0!, {{r4-r11, lr}}", // discard "fake" sw frame built during initialization and update r0, load default LR
        "msr psp, r0",               // set psp to r0 (hw frame)
        "isb",                       // flush cpu pipeline, needed because we overwritten stack pointer

        "bx lr", // start os and switch to PSP mode
    );
}

#[exception]
fn SysTick() {
    Kernel::lock(|k| {
        k.on_tick();
    });

    SCB::set_pendsv(); // handle switch in PendSV
}