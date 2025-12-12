#![no_std]
#![no_main]

use embedded_hal::digital::StatefulOutputPin;
use whyos_demo::{Board, hal};

use defmt::info;

type LedPin = hal::gpio::Pin<
    hal::gpio::bank0::Gpio22,
    hal::gpio::FunctionSio<hal::gpio::SioOutput>,
    hal::gpio::PullDown
>;

static LED: whyos::Mutex<Option<LedPin>> = whyos::Mutex::new(None);

#[unsafe(no_mangle)]
extern "C" fn blinky_task() -> ! {
    loop {
        {
            let mut led_opt = LED.lock();

            if let Some(led) = led_opt.as_mut() {
                led.toggle().unwrap();
                info!("blink!");
            }
        }

        whyos::sleep(500);
    }
}

#[hal::entry]
fn main() -> ! {
    let board = Board::init();
    let mut syst = board.syst;
    let freq = board.sys_freq;

    let led = board.pins.gpio22.into_push_pull_output();
    {
        let mut guard = LED.lock();
        *guard = Some(led);
    }

    whyos::TaskBuilder::new(blinky_task).priority(2).stack_size(2048).spawn().unwrap();


    defmt::info!("Starting WhyOS");
    unsafe { whyos::start(&mut syst, freq / 1000); }
}