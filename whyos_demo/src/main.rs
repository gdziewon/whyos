#![no_std]
#![no_main]

use whyos_demo::{board, hal};
use whyos::Mutex;

use defmt::info;

static WORKER_NUM: Mutex<u32> = Mutex::new(0);

#[unsafe(no_mangle)]
extern "C" fn manager_task() -> ! {
    info!("manager: start");
    loop {
        info!("manager: What's up worker?");
        whyos::add_task(worker_task, 2, 2048).unwrap();
        whyos::sleep(2000);
        info!("manager: He was a good man, Rest in peace worker nr {}", *WORKER_NUM.lock());
        *WORKER_NUM.lock() += 1;
    }
}

#[unsafe(no_mangle)]
extern "C" fn worker_task() -> ! {
    info!("worker: Hi! I'm worker nr {}. I am alive and well, thank you for asking", *WORKER_NUM.lock());
    whyos::sleep(500);
    info!("worker: I should leave anyway, see you soon");
    whyos::exit();
}

#[hal::entry]
fn main() -> ! {
    let (mut syst, freq) = board::init();

    whyos::add_task(manager_task, 1, 4096).unwrap();

    defmt::info!("Starting WhyOS");
    unsafe { whyos::start(&mut syst, freq / 1000); }
}