mod idle;
mod kernel;
mod panic;

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

    #[inline]
    pub fn try_lock<R>(kernel_op: impl FnOnce(&mut Self) -> R) -> Option<R> {
        critical_section::with(|cs| {
            if let Ok(mut kernel) = KERNEL.borrow(cs).try_borrow_mut() {
                Some(kernel_op(&mut kernel))
            } else {
                None
            }
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

#[inline(always)]
pub fn yield_now() {
    crate::arch::yield_now();
}
