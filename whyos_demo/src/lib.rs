#![no_std]

pub mod board;
pub mod rt;

#[cfg(feature = "shell")]
pub mod shell;

pub use rp235x_hal as hal;

use defmt_rtt as _; // init rtt

use core::sync::atomic::{self, Ordering};

pub fn halt() -> ! {
    loop { atomic::compiler_fence(Ordering::SeqCst); }
}

pub fn assert_print(cond: bool) {
    if cond {
        defmt::info!("OK");
    } else {
        defmt::error!("FAILED");
    }
}