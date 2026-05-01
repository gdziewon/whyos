use core::panic::PanicInfo;
use core::arch::asm;
use crate::{kill, yield_cpu};
use crate::scheduler::Kernel;

#[inline(always)]
fn read_ipsr() -> u32 {
    let ipsr: u32;
    unsafe {
        asm!("mrs {}, ipsr", out(reg) ipsr);
    }
    ipsr
}

#[inline(never)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let is_task = read_ipsr() == 0;

    if is_task {
        if let Some(tid) = Kernel::lock(|k| k.current_task()) {
            defmt::warn!("WhyOS: Task {} panicked: {}", tid.id(), info);

            unsafe { let _ = kill(tid); }
            yield_cpu();

            loop {
                cortex_m::asm::wfi();
            }
        } else {
            defmt::error!("WhyOS: Idle task panic: {}", info);
            loop {
                cortex_m::asm::bkpt();
            }
        }
    } else {
        defmt::error!("WhyOS: KERNEL PANIC: {}", info);
        loop {
            cortex_m::asm::bkpt();
        }
    }
}