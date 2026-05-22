#![no_std]
#![no_main]

use core::sync::atomic::{self, Ordering};
use defmt_rtt as _;
use rp235x_hal as hal;

use whyos::Queue;
use whyos_demo::board::Board;

static BENCH_QUEUE: Queue<u32, 10> = Queue::new();

extern "C" fn bench_runner() {
    defmt::info!("=== BENCHMARK: Queue Fast-Path ===");

    let mut min_send: u32 = u32::MAX;
    let mut min_recv: u32 = u32::MAX;
    let mut total_send: u64 = 0;
    let mut total_recv: u64 = 0;
    const ITERS: u32 = 1000;

    for i in 0..ITERS {
        let start = whyos::cycle_count();
        BENCH_QUEUE.try_send(i).unwrap();
        let mid = whyos::cycle_count();
        let _v = BENCH_QUEUE.receive();
        let end = whyos::cycle_count();

        let s_cycles = mid.wrapping_sub(start);
        let r_cycles = end.wrapping_sub(mid);

        if i > 0 {
            if s_cycles < min_send { min_send = s_cycles; }
            if r_cycles < min_recv { min_recv = r_cycles; }
            total_send += s_cycles as u64;
            total_recv += r_cycles as u64;
        }
    }

    let avg_send = (total_send / (ITERS - 1) as u64) as u32;
    let avg_recv = (total_recv / (ITERS - 1) as u64) as u32;
    defmt::info!("Send - Min: {} cycles | Avg: {} cycles", min_send, avg_send);
    defmt::info!("Recv - Min: {} cycles | Avg: {} cycles", min_recv, avg_recv);

    cortex_m_semihosting::debug::exit(cortex_m_semihosting::debug::EXIT_SUCCESS);
    loop { atomic::compiler_fence(Ordering::SeqCst); }
}

#[hal::entry]
fn main() -> ! {
    let _board = Board::init();

    whyos::TaskBuilder::new(bench_runner).priority(2).name("Runner").spawn().unwrap();

    whyos::start(whyos::Freq::ONE_KHZ);
}