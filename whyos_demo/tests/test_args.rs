#![no_std]
#![no_main]

use defmt::info;
use rp235x_hal::halt;
use whyos_demo::{board::Board, hal, assert_print};
use cortex_m_semihosting::debug;

static TASK_ARG: whyos::Mutex<u32> = whyos::Mutex::new(0);

extern "C" fn supervisor_task() -> ! {
    whyos::spawn_with_priority(test_value, 2).unwrap();
    whyos::sleep(200);
    whyos::spawn_with_priority(test_static_mut, 2).unwrap();
    whyos::sleep(200);
    whyos::spawn_with_priority(test_ptr_mut, 2).unwrap();
    whyos::sleep(200);
    debug::exit(debug::EXIT_SUCCESS);
    halt();
}

extern "C" fn test_value() -> ! {
    info!("test_value:");
    extern "C" fn takes_value(a: u32) -> ! {
        *(TASK_ARG.lock()) = a;
        whyos::exit();
    }

    let arg: u32 = 65;

    whyos::TaskBuilder::with_value(takes_value, arg).spawn().unwrap();
    whyos::sleep(100);

    assert_print(*(TASK_ARG.lock()) == arg);
    whyos::exit();
}

extern "C" fn test_ptr_mut() -> ! {
    info!("test_ptr_mut:");
    extern "C" fn takes_ptr_mut(a: *mut u32) -> ! {
        *(TASK_ARG.lock()) = unsafe { *a };
        whyos::exit();
    }

    static mut ARG: u32 = 42;
    let arg_ptr = &raw mut ARG;

    unsafe { whyos::TaskBuilder::with_ptr_mut(takes_ptr_mut, arg_ptr).spawn().unwrap() };
    whyos::sleep(100);

    assert_print(*(TASK_ARG.lock()) == unsafe { ARG });
    whyos::exit();
}

extern "C" fn test_static_mut() -> ! {
    info!("test_static_mut:");
    extern "C" fn takes_static_mut(a: &'static mut [u32; 3]) -> ! {
        *(TASK_ARG.lock()) = a[1];
        whyos::exit();
    }

    let buf_idx1 = 4;
    let buffer: &'static mut [u32; 3] = cortex_m::singleton!(: [u32; 3] = [1, buf_idx1, 3]).unwrap();
    whyos::TaskBuilder::with_static_mut(takes_static_mut, buffer).spawn().unwrap();
    whyos::sleep(100);

    assert_print(*(TASK_ARG.lock()) == buf_idx1);
    whyos::exit();
}



#[hal::entry]

fn main() -> ! {
    let mut board = Board::init();

    whyos::TaskBuilder::new(supervisor_task).priority(1).stack_size(4096).spawn().unwrap();
    unsafe { whyos::start(&mut board.syst, board.sys_freq / 1000); }
}