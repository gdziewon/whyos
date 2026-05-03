use super::trap::trap_entry;
use riscv::register::mie;

pub struct SioTimer;
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
}

pub unsafe fn start_os(tick_hz: u32) -> ! { // todo: add Hertz struct?
    unsafe { riscv::register::mtvec::write(
        trap_entry as *const () as usize,
        riscv::register::mtvec::TrapMode::Direct
    )};

    let interval_us = (1_000_000 / tick_hz) as u64; // watchdog tick is 1 MHZ on RP2350

    let now = SioTimer::get_time();
    SioTimer::set_compare(now + interval_us);

    unsafe { mie::set_mtimer(); }// enable timer interrupt

    let idle_sp = crate::scheduler::Kernel::lock(|k| {
        k.set_timer_interval(tick_hz);
        k.idle_sp()
    });

    unsafe {
        core::arch::asm!(
            "mv sp, {0}",
            "j restore_context", // jump to the label in start
            in(reg) idle_sp,
            options(noreturn)
        );
    }
}