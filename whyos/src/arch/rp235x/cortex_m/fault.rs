use cortex_m_rt::{exception, ExceptionFrame};
use crate::reboot;
use crate::scheduler::Kernel;

#[exception]
unsafe fn HardFault(ef: &ExceptionFrame) -> ! {
    let tid = Kernel::try_lock(|k| k.current_task()).flatten();

    if let Some(t) = tid {
        defmt::error!("WhyOS: HARD FAULT triggered while running Task {}!", t.id());
    } else {
        defmt::error!("WhyOS: HARD FAULT triggered in Kernel or unknown context!");
    }

    defmt::error!(
        "Registers:
        r0={:X} r1={:X} r2={:X} r3={:X}
        r12={:X} lr={:X} pc={:X} xpsr={:X}",
        ef.r0(), ef.r1(), ef.r2(), ef.r3(),
        ef.r12(), ef.lr(), ef.pc(), ef.xpsr()
    );
    defmt::warn!("Rebooting system...");
    cortex_m::asm::delay(15_000_000);
    reboot()
}