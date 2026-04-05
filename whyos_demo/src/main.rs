#![no_std]
#![no_main]

use whyos_demo::{board::{Board, LedPin}, hal};
use defmt::info;
use embedded_hal::digital::StatefulOutputPin as _;

#[cfg(feature = "shell")]
use whyos_demo::shell;

static mut LED_STORAGE: Option<LedPin> = None;

#[unsafe(no_mangle)]
extern "C" fn blinky_task(led_opt: &'static mut Option<LedPin>) {
    let led = led_opt.as_mut().expect("LED not initialized!");

    loop {
        let _ = led.toggle();
        whyos::sleep(200);
    }
}

#[hal::entry]
fn main() -> ! {
    let mut board = Board::init();

    unsafe {
        LED_STORAGE = Some(board.led);
        let led_ref = &mut *(&raw mut LED_STORAGE);

        whyos::TaskBuilder::with_static_mut(blinky_task, led_ref)
            .name("blinky")
            .spawn()
            .expect("Failed to spawn blinky");
    }


    #[cfg(feature = "shell")]
    shell::init_shell(board.uart);

    info!("Starting WhyOS...");
    unsafe { whyos::start(&mut board.syst, board.sys_freq / 1000); }
}