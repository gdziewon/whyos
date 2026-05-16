#[cfg(target_arch = "arm")]
pub mod cortex_m;
#[cfg(target_arch = "arm")]
pub use cortex_m::*;

#[cfg(target_arch = "riscv32")]
pub mod riscv32;
#[cfg(target_arch = "riscv32")]
pub use riscv32::*;

pub mod soc;

use crate::scheduler::Kernel;
use crate::utils::log;

#[allow(dead_code)]
pub trait KernelArch {
    const HEAP_KB: usize;

    unsafe fn init(tick_hz: u32);
    fn tick(interval_hz: u32);
    fn set_tick_freq(freq: u32);
    unsafe fn start() -> !;
}

pub type TargetArch = soc::SocArch;

pub unsafe fn start_os(freq: u32) -> ! {
    log::debug!("Arch start");
    unsafe {
        TargetArch::init(freq);
        TargetArch::start()
    }
}

#[unsafe(no_mangle)]
extern "C" fn get_idle_task_sp() -> usize {
    Kernel::lock(|k| k.idle_sp())
}