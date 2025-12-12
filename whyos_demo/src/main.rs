#![no_std]
#![no_main]

use whyos_demo::{board, hal};

use defmt::info;

#[unsafe(no_mangle)]
extern "C" fn manager_task() -> ! {
    let mr_big_array: [u32; 305] = [0xDEADBEEF; 305];
    let mut x = 1.5;
    loop {
        x += 0.5;
        info!("x is {}", x);
        whyos::sleep(500);
    }
}

#[unsafe(no_mangle)]
extern "C" fn worker_task() -> ! {
    loop {
        let y = 1.6;
        info!("y is {}", y);
        whyos::sleep(500);
    }
}

#[hal::entry]
fn main() -> ! {
    let (mut syst, freq) = board::init();

    whyos::TaskBuilder::new(worker_task).priority(1).stack_size(2048).spawn().unwrap();
    whyos::TaskBuilder::new(manager_task).priority(2).stack_size(2048).spawn().unwrap();


    defmt::info!("Starting WhyOS");
    unsafe { whyos::start(&mut syst, freq / 1000); }
}