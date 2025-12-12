use rp235x_hal::{self as hal, Clock as _};
use cortex_m::peripheral::SYST;

const XTAL_FREQ_HZ: u32 = 12_000_000u32; // 12 MHz

pub struct Board {
    pub syst: SYST,
    pub sys_freq: u32,
    pub pins: hal::gpio::Pins,
    pub resets: hal::pac::RESETS,
}

impl Board {
    pub fn init() -> Self {
        let mut pac = hal::pac::Peripherals::take().expect("PAC taken");
        let core = cortex_m::Peripherals::take().expect("Core taken");

        let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

        let clocks = hal::clocks::init_clocks_and_plls(
            XTAL_FREQ_HZ,
            pac.XOSC,
            pac.CLOCKS,
            pac.PLL_SYS,
            pac.PLL_USB,
            &mut pac.RESETS,
            &mut watchdog,
        ).ok().unwrap();

        let sys_freq = clocks.system_clock.freq().to_Hz();

        let sio = hal::Sio::new(pac.SIO);
        let pins = hal::gpio::Pins::new(
            pac.IO_BANK0,
            pac.PADS_BANK0,
            sio.gpio_bank0,
            &mut pac.RESETS,
        );

        Self {
            syst: core.SYST,
            sys_freq,
            pins,
            resets: pac.RESETS,
        }
    }
}