#![no_std]
#![no_main]

use core::sync::atomic::{AtomicBool, Ordering};
use rp235x_hal::halt;
use whyos_demo::{board::Board, hal};
use cortex_m_semihosting::debug;

static FLAG: AtomicBool = AtomicBool::new(false);

#[unsafe(no_mangle)]
extern "C" fn supervisor_task() -> ! {
    defmt::info!("Priority test:");
    whyos::spawn_with_priority(low_prio, 2).unwrap();
    whyos::sleep(100);


    if FLAG.load(Ordering::SeqCst) {
        defmt::info!("OK");
        debug::exit(debug::EXIT_SUCCESS);
    } else {
        defmt::error!("FAILED");
        debug::exit(debug::EXIT_FAILURE);
    }

    halt()
}


#[unsafe(no_mangle)]
extern "C" fn low_prio() -> ! {
    FLAG.store(true, Ordering::SeqCst);
    whyos::exit();
}


#[hal::entry]

fn main() -> ! {
    let mut board = Board::init();

    whyos::TaskBuilder::new(supervisor_task).priority(1).stack_size(4096).spawn().unwrap();

    unsafe { whyos::start(&mut board.syst, board.sys_freq / 1000); }
}