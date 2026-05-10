pub(super) struct SioTimer;

impl SioTimer { // 3.1.8. RISC-V platform timer
    const BASE: usize = 0xd000_0000;

    const MTIME: *mut u32 = (Self::BASE + 0x1B0) as *mut u32; // taken from rp2350 datasheet, wont work on every riscv!!!
    const MTIMEH: *mut u32 = (Self::BASE + 0x1B4) as *mut u32;

    const MTIMECMP: *mut u32 = (Self::BASE + 0x1B8) as *mut u32;
    const MTIMECMPH: *mut u32 = (Self::BASE + 0x1BC) as *mut u32;

    pub fn get_time() -> u64 {
        unsafe {
            loop { // to make sure we dont read exactly on timer tick, getting some weird value
                let hi = Self::MTIMEH.read_volatile();
                let lo = Self::MTIME.read_volatile();
                let hi2 = Self::MTIMEH.read_volatile();
                if hi == hi2 {
                    return ((hi as u64) << 32) | (lo as u64);
                }
            }
        }
    }

    pub fn set_compare(cmp: u64) {
        unsafe {
            Self::MTIMECMPH.write_volatile(0xFFFF_FFFF); // to not risk triggering interrupt
            Self::MTIMECMP.write_volatile(cmp as u32);
            Self::MTIMECMPH.write_volatile((cmp >> 32) as u32);
        }
    }

    pub fn schedule_next(interval_hz: u32) {
        let interval_us = (1_000_000 / interval_hz) as u64; // ARCH 1_000_000 specific for rp235x
        let now = Self::get_time();
        Self::set_compare(now + interval_us);
    }
}

