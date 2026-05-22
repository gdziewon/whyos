#![no_std]

pub const SYS_CLOCK_MHZ: u32 = 150;

#[inline(always)]
pub fn cycles_to_ns(cycles: u32, clock_mhz: u32) -> u32 {
    ((cycles as u64 * 1000) / clock_mhz as u64) as u32
}