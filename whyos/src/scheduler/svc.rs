use core::arch::naked_asm;

#[unsafe(no_mangle)]
extern "C" fn reject_bootstrap() -> ! {
    panic!("WhyOS: OS API misused. start() called from an active task (PSP mode)!");
}

#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn SVCall() {
    naked_asm!(
        // check if we are running in PSP mode
        "tst lr, #4",
        "bne reject_bootstrap", // If we were on PSP, somebody called start() from running task, panic

        // setup PSP
        "bl get_idle_task_sp",       // returns idle sp in r0
        "ldmia r0!, {{r4-r11, lr}}", // discard "fake" sw frame built during initialization and update r0, load default LR
        "msr psp, r0",               // set psp to r0 (hw frame)
        "isb",                       // flush cpu pipeline, needed because we overwritten stack pointer

        "bx lr", // start os and switch to PSP mode
    );
}