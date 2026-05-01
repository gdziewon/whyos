use rp235x_hal::{
    self as hal, Clock as _, fugit::RateExtU32 as _,
    gpio::{
        bank0::{Gpio0, Gpio1, Gpio22},
        FunctionUart, FunctionSioOutput, Pin, PullDown
    },
    pac::{UART0, RESETS},
    uart::{DataBits, StopBits, UartConfig, UartPeripheral, Reader, Writer}
};
use whyos::cortex_m::peripheral::SYST;

const XTAL_FREQ_HZ: u32 = 12_000_000u32;

pub type UartRx = Reader<UART0, (Pin<Gpio0, FunctionUart, PullDown>, Pin<Gpio1, FunctionUart, PullDown>)>;
pub type UartTx = Writer<UART0, (Pin<Gpio0, FunctionUart, PullDown>, Pin<Gpio1, FunctionUart, PullDown>)>;
pub type LedPin = Pin<Gpio22, FunctionSioOutput, PullDown>;

pub struct Board {
    pub syst: SYST,
    pub sys_freq: u32,
    pub resets: RESETS,
    pub uart: Uart,
    pub led: LedPin, // just a LED I hooked up on GPIO22
}

pub struct Uart {
    pub rx: UartRx,
    pub tx: UartTx
}

impl Board {
    pub fn init() -> Self {
        let mut pac = hal::pac::Peripherals::take().expect("PAC taken");
        let core = whyos::cortex_m::Peripherals::take().expect("Core taken");

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

        let uart_pins = (
            pins.gpio0.into_function::<FunctionUart>(),
            pins.gpio1.into_function::<FunctionUart>(),
        );

        let led = pins.gpio22.into_push_pull_output();

        let mut uart = UartPeripheral::new(pac.UART0, uart_pins, &mut pac.RESETS)
            .enable(
                UartConfig::new(115200.Hz(), DataBits::Eight, None, StopBits::One),
                sys_freq.Hz(),
            )
            .unwrap();

        uart.enable_rx_interrupt();

        let (rx, tx) = uart.split();

        Self {
            syst: core.SYST,
            sys_freq,
            resets: pac.RESETS,
            uart: Uart { rx, tx },
            led,
        }
    }
}