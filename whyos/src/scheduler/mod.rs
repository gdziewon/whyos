mod idle;
mod preempt;
mod kernel;
mod svc;

use core::cell::RefCell;

use critical_section::Mutex;
pub use kernel::{Kernel, MAX_TASKS, TaskMask};

static KERNEL: Mutex<RefCell<Kernel>> = Mutex::new(RefCell::new(Kernel::new()));

impl Kernel {
    #[inline]
    pub fn lock<R>(kernel_op: impl FnOnce(&mut Self) -> R) -> R {
        critical_section::with(|cs| {
            let mut kernel = KERNEL.borrow_ref_mut(cs);
            kernel_op(&mut kernel)
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ContextSwitch {
    Yield,
    Continue,
}

impl ContextSwitch {
    #[inline]
    pub const fn yield_if(condition: bool) -> Self {
        if condition {
            ContextSwitch::Yield
        } else {
            ContextSwitch::Continue
        }
    }
}

pub fn config_systick(syst: &mut cortex_m::peripheral::SYST, freq: u32) {
    syst.set_clock_source(cortex_m::peripheral::syst::SystClkSource::Core);
    syst.set_reload(freq);
    syst.clear_current();
    syst.enable_counter();
    syst.enable_interrupt();
}

#[inline(always)]
pub fn yield_now() {
    cortex_m::peripheral::SCB::set_pendsv();
}
