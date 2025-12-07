#![no_std]

pub mod board;
pub mod rt;

pub use rp235x_hal as hal;

use defmt_rtt as _; // init rtt

use core::sync::atomic::{self, Ordering};

pub fn halt() -> ! {
    loop { atomic::compiler_fence(Ordering::SeqCst); }
}