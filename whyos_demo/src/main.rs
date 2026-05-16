#![no_std]
#![no_main]

use whyos_demo::{board::{Board, LedPin}};
use embedded_hal::digital::StatefulOutputPin as _;
use whyos_demo::hal::entry;


extern "C" fn blinky_task(leds: &'static whyos::Mutex<Option<LedPin>>) {

    loop {
        let mut led_slot = leds.lock();
        let led = led_slot.as_mut()
            .expect("LED not initialized!");

        let _ = led.toggle();
        whyos::sleep(200);
    }
}

#[entry]
fn main() -> ! {
    let board = Board::init();

    static LED_STORAGE: whyos::Mutex<Option<LedPin>> = whyos::Mutex::new(None);
    *LED_STORAGE.lock() = Some(board.led);

    whyos::TaskBuilder::with_static_ref(blinky_task, &LED_STORAGE)
        .name("blinky")
        .spawn()
        .expect("Failed to spawn blinky");


    #[cfg(feature = "shell")]
    whyos_demo::shell::init_shell(board.uart);

    whyos::start(whyos::Freq::ONE_KHZ);
}