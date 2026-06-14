#![no_std]
#![no_main]

use defmt_rtt as _;
use rp235x_hal as hal;
use core::sync::atomic::{self, Ordering};

use whyos::yield_cpu;
use whyos_demo::board::Board;

const MAX_ITERS: u32 = 1000000;

const SIO_BASE: u32 = 0xd0000000;
const GPIO_OUT_SET: u32 = SIO_BASE + 0x018;
const GPIO_OUT_CLR: u32 = SIO_BASE + 0x020;
const PIN_MASK: u32 = 1 << 15;

#[inline(always)]
fn pin_high() {
    unsafe { core::ptr::write_volatile(GPIO_OUT_SET as *mut u32, PIN_MASK) };
}

#[inline(always)]
fn pin_low() {
    unsafe { core::ptr::write_volatile(GPIO_OUT_CLR as *mut u32, PIN_MASK) };
}

extern "C" fn task_ping() {
    defmt::info!("=== BENCHMARK: Pure Hardware Context Switch ===");
    defmt::info!("Connect Logic Analyzer to GPIO 15. Running...");

    for _ in 0..MAX_ITERS {
        pin_high();
        yield_cpu();
    }

    defmt::info!("Benchmark finished.");
    cortex_m_semihosting::debug::exit(cortex_m_semihosting::debug::EXIT_SUCCESS);
    loop { atomic::compiler_fence(Ordering::SeqCst); }
}

extern "C" fn task_pong() {
    for _ in 0..MAX_ITERS {
        pin_low();
        yield_cpu();
    }
}

#[hal::entry]
fn main() -> ! {
    let _board = Board::init();

    whyos::TaskBuilder::new(task_ping).priority(6).name("Ping").spawn().unwrap();
    whyos::TaskBuilder::new(task_pong).priority(6).name("Pong").spawn().unwrap();

    whyos::start(whyos::Freq::ONE_HZ);
}