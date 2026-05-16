#![no_std]
#![no_main]

use whyos_demo::{board::{Board, LedPin}};
use embedded_hal::digital::StatefulOutputPin as _;
use whyos_demo::hal::entry;

static mut LED_STORAGE: Option<LedPin> = None;

#[unsafe(no_mangle)]
extern "C" fn blinky_task(led_opt: &'static mut Option<LedPin>) {
    let led = led_opt.as_mut().expect("LED not initialized!");

    loop {
        let _ = led.toggle();
        whyos::sleep(200);
    }
}

#[entry]
fn main() -> ! {
    let board = Board::init();

    unsafe {
        LED_STORAGE = Some(board.led);
        let led_ref = &mut *(&raw mut LED_STORAGE);

        whyos::TaskBuilder::with_static_mut(blinky_task, led_ref)
            .name("blinky")
            .spawn()
            .expect("Failed to spawn blinky");
    }

    #[cfg(feature = "shell")]
    whyos_demo::shell::init_shell(board.uart);

    whyos::start(whyos::Freq::ONE_KHZ);
}