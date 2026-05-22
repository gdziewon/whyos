#![no_std]
#![no_main]

use core::sync::atomic::{self, AtomicU32, Ordering};
use defmt_rtt as _;
use rp235x_hal as hal;

use whyos::yield_cpu;
use whyos_bench::{cycles_to_ns, SYS_CLOCK_MHZ};
use whyos_demo::board::Board;

static SWITCH_START: AtomicU32 = AtomicU32::new(0);

const MAX_ITERS: u32 = 1000;

extern "C" fn task_ping() {
    defmt::info!("=== BENCHMARK: Cooperative Context Switch ===");

    for _ in 0..MAX_ITERS {
        SWITCH_START.store(whyos::cycle_count(), Ordering::Relaxed);

        yield_cpu();
    }
}

extern "C" fn task_pong() {
    let mut min_cycles: u32 = u32::MAX;
    let mut total_cycles: u64 = 0;

    for iter in 0..MAX_ITERS {
        let end = whyos::cycle_count();
        let start = SWITCH_START.load(Ordering::Relaxed);

        let diff = end.wrapping_sub(start);

        if iter > 0 {
            if diff < min_cycles { min_cycles = diff; }
            total_cycles += diff as u64;
        }

        yield_cpu();
    }

    let avg = (total_cycles / (MAX_ITERS - 1) as u64) as u32;
    let ns_avg = cycles_to_ns(avg, SYS_CLOCK_MHZ);
    let ns_min = cycles_to_ns(min_cycles, SYS_CLOCK_MHZ);
    defmt::info!("Result: Min {} cycles ({} ns) | Avg {} cycles ({} ns)", min_cycles, ns_min, avg, ns_avg);

    cortex_m_semihosting::debug::exit(cortex_m_semihosting::debug::EXIT_SUCCESS);
    loop { atomic::compiler_fence(Ordering::SeqCst); }
}

#[hal::entry]
fn main() -> ! {
    let _board = Board::init();

    whyos::TaskBuilder::new(task_ping).priority(6).name("Ping").spawn().unwrap();
    whyos::TaskBuilder::new(task_pong).priority(6).name("Pong").spawn().unwrap();

    whyos::start(whyos::Freq::ONE_HZ);
}