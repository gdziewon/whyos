use core::panic::PanicInfo;

use rp235x_hal as hal;
use hal::{binary_info, block::ImageDef};
use cortex_m_rt::{exception, ExceptionFrame};
use defmt::error;

use crate::halt;

#[inline(never)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! { // inspired by 'panic_halt' implementation
    error!("PANIC! {}", info);
    halt();
}

#[exception]
unsafe fn HardFault(ef: &ExceptionFrame) -> ! {
    error!(
        "HardFault: r0={} r1={} r2={} r3={} r12={} lr={} pc={} xpsr={}",
        ef.r0(),
        ef.r1(),
        ef.r2(),
        ef.r3(),
        ef.r12(),
        ef.lr(),
        ef.pc(),
        ef.xpsr()
    );
    halt();
}

#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: ImageDef = ImageDef::secure_exe(); // costruct secure exe header

// just a small description of the binary
#[unsafe(link_section = ".bi_entries")]
#[used]
pub static BINARY_ENTRIES: [binary_info::EntryAddr; 5] = [
    binary_info::rp_program_name!(c"WhyOS"),
    binary_info::rp_cargo_version!(),
    binary_info::rp_program_description!(c"RTOS Demo"),
    binary_info::rp_cargo_homepage_url!(),
    binary_info::rp_program_build_attribute!(),
];