#![no_std]
#![no_main]

use core::sync::atomic::{self, Ordering};
use defmt_rtt as _;
use rp235x_hal as hal;

use whyos::Mutex;
use whyos_demo::board::Board;

static BENCH_MUTEX: Mutex<u32> = Mutex::new(0);

extern "C" fn bench_runner() {
    defmt::info!("=== BENCHMARK: Mutex Uncontended ===");

    let mut total_lock: u64 = 0;
    let mut total_unlock: u64 = 0;
    let mut min_lock: u32 = u32::MAX;
    let mut min_unlock: u32 = u32::MAX;
    const ITERS: u32 = 1000;

    for iter in 0..ITERS {
        let start = whyos::cycle_count();
        let g = BENCH_MUTEX.lock();
        let mid = whyos::cycle_count();
        drop(g);
        let end = whyos::cycle_count();

        let l_cycles = mid.wrapping_sub(start);
        let u_cycles = end.wrapping_sub(mid);

        if iter > 0 {
            if l_cycles < min_lock { min_lock = l_cycles; }
            if u_cycles < min_unlock { min_unlock = u_cycles; }
            total_lock += l_cycles as u64;
            total_unlock += u_cycles as u64;
        }
    }

    let avg_lock = (total_lock / (ITERS - 1) as u64) as u32;
    let avg_unlock = (total_unlock / (ITERS - 1) as u64) as u32;
    defmt::info!("Lock   - Min: {} cycles | Avg: {} cycles", min_lock, avg_lock);
    defmt::info!("Unlock - Min: {} cycles | Avg: {} cycles", min_unlock, avg_unlock);

    cortex_m_semihosting::debug::exit(cortex_m_semihosting::debug::EXIT_SUCCESS);
    loop { atomic::compiler_fence(Ordering::SeqCst); }
}

#[hal::entry]
fn main() -> ! {
    let _board = Board::init();

    whyos::TaskBuilder::new(bench_runner).priority(2).name("Runner").spawn().unwrap();

    whyos::start(whyos::Freq::ONE_KHZ);
}