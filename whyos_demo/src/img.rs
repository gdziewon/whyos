extern crate whyos; // for exception macro to work

use crate::hal::{binary_info, block::ImageDef};


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