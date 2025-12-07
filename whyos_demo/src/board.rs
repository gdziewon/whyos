use rp235x_hal::{self as hal, Clock as _};
use cortex_m::peripheral::SYST;

const XTAL_FREQ_HZ: u32 = 12_000_000u32; // 12 MHz

pub fn init() -> (SYST, u32) {
    let mut pac = hal::pac::Peripherals::take().expect("Failed to take PAC Peripherals");
    let core = cortex_m::Peripherals::take().expect("Failed to take Cortex Peripherals");

    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    let clocks = hal::clocks::init_clocks_and_plls(
        XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    ).expect("Failed to initialize clocks");

    let sys_freq = clocks.system_clock.freq().to_Hz();

    (core.SYST, sys_freq)
}