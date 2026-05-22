#[cfg(target_arch = "riscv32")]
mod timer;

use crate::arch::KernelArch;
use crate::utils::log;

pub struct SocArch;

#[cfg(target_arch = "arm")]
impl KernelArch for SocArch {
    const HEAP_KB: usize = 256;

    unsafe fn init(tick_hz: u32) {


        log::trace!("Arch initializing SysTick with {} Hz", tick_hz);
        use cortex_m::peripheral::syst::SystClkSource;
        let mut core = unsafe { cortex_m::Peripherals::steal() };

        #[cfg(feature = "profiler")]
        crate::arch::init_cycle_counter(&mut core);

        let syst = &mut core.SYST;
        let interval_us = 1_000_000 / tick_hz; // watchdog tick = 1 MHz
        syst.set_clock_source(SystClkSource::External);
        syst.set_reload(interval_us);
        syst.clear_current();
        syst.enable_counter();
        syst.enable_interrupt();
    }

    #[inline(always)]
    fn tick(_interval_hz: u32) {} // systick auto-reloads

    fn set_tick_freq(freq: u32) {
        log::trace!("Arch change system freq to {} Hz", freq);
        let mut core = unsafe { cortex_m::Peripherals::steal() };
        let syst = &mut core.SYST;
        let interval_us = 1_000_000 / freq;
        syst.set_reload(interval_us);
        syst.clear_current();
    }

    unsafe fn start() -> ! {
        log::trace!("Arch calling SVC to bootstrap scheduler");
        unsafe { core::arch::asm!("svc 0", options(noreturn)) }
    }
}

#[cfg(target_arch = "riscv32")]
impl KernelArch for SocArch {
    const HEAP_KB: usize = 256;

    unsafe fn init(tick_hz: u32) {
        log::trace!("Arch initializing trap handler and SioTimer with {} Hz", tick_hz);
        unsafe {
            riscv::register::mtvec::write(
                crate::arch::trap_entry as *const () as usize,
                riscv::register::mtvec::TrapMode::Direct,
            );
        }
        timer::SioTimer::schedule_next(tick_hz);
        unsafe { riscv::register::mie::set_mtimer() };
    }

    fn tick(interval_hz: u32) {
        timer::SioTimer::schedule_next(interval_hz);
    }

    fn set_tick_freq(freq: u32) {
        log::trace!("Arch change system freq to {} Hz", freq);
        timer::SioTimer::schedule_next(freq);
    }

    unsafe fn start() -> ! {
        log::trace!("Arch scheduler bootstrap");
        unsafe {
            core::arch::asm!(
                "call get_idle_task_sp",
                "mv sp, a0",
                "j restore_context", // assumes there is restore_context somehwere
                options(noreturn)
            )
        }
    }
}