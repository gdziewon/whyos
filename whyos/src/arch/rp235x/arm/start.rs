use core::arch::{naked_asm, asm};

fn config_systick(syst: &mut cortex_m::peripheral::SYST, tick_hz: u32) {
    let interval_us = 1_000_000 / tick_hz; // ARCH: watchdog tick is 1 MHZ on RP2350

    syst.set_clock_source(cortex_m::peripheral::syst::SystClkSource::External);
    syst.set_reload(interval_us);
    syst.clear_current();
    syst.enable_counter();
    syst.enable_interrupt();
}

pub unsafe fn start_os(tick_hz: u32) -> ! {
    let mut core = unsafe { cortex_m::Peripherals::steal() };
    config_systick(&mut core.SYST, tick_hz);

    unsafe {
        asm!(
            "svc 0",
            options(noreturn)
        );
    }
}

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