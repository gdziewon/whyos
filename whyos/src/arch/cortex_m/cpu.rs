use core::arch::asm;


#[inline(always)]
pub fn bkpt() { cortex_m::asm::bkpt(); }

#[inline(always)]
pub fn wfi() { cortex_m::asm::wfi(); }

#[inline(always)]
pub fn yield_now() { cortex_m::peripheral::SCB::set_pendsv(); }

#[inline(always)]
pub fn reset() -> ! { cortex_m::peripheral::SCB::sys_reset(); }

#[inline(always)]
pub fn is_in_task() -> bool {
    let ipsr: u32;
    unsafe {
        asm!(
            "mrs {}, ipsr",
            out(reg) ipsr
        );
    }
    ipsr == 0
}

pub fn init_cycle_counter(core: &mut cortex_m::Peripherals) {
    core.DCB.enable_trace();
    cortex_m::peripheral::DWT::unlock();
    core.DWT.enable_cycle_counter();
}

#[inline(always)]
pub fn cycle_count() -> u32 {
    cortex_m::peripheral::DWT::cycle_count()
}