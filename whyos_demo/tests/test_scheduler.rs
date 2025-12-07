#![no_std]
#![no_main]

use core::sync::atomic::{AtomicBool, Ordering};
use rp235x_hal::halt;
use whyos_demo::{board, hal};
use cortex_m_semihosting::debug;

static FLAG: AtomicBool = AtomicBool::new(false);

#[unsafe(no_mangle)]
extern "C" fn supervisor_task() -> ! {
    whyos::add_task(low_prio, 2, 2048).unwrap();
    whyos::sleep(100);


    if FLAG.load(Ordering::SeqCst) {
        defmt::info!("Priority Test OK!");
        debug::exit(debug::EXIT_SUCCESS);
    } else {
        defmt::error!("Low priority task never ran!!");
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
    let (mut syst, freq) = board::init();

    whyos::add_task(supervisor_task, 1, 4096).unwrap();

    defmt::info!("Starting WhyOS");
    unsafe { whyos::start(&mut syst, freq / 1000); }
}