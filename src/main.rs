#![no_std]
#![no_main]

mod kernel;

use core::panic::PanicInfo;
use core::sync::atomic::{self, Ordering};

use rp235x_hal::{self as hal, Clock as _};
use hal::{binary_info, block::ImageDef};
use cortex_m::{self, peripheral};
use cortex_m_rt::{exception, ExceptionFrame};
use defmt::{info, error};
use defmt_rtt as _;

#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: ImageDef = ImageDef::secure_exe();

const XTAL_FREQ_HZ: u32 = 12_000_000u32; // 12 MHz

static STACK_1: kernel::Stack<4096> = kernel::Stack::<4096>::new();
static STACK_2: kernel::Stack<4096> = kernel::Stack::<4096>::new();

#[unsafe(no_mangle)]
extern "C" fn task_1() -> ! {
    loop {
        info!("I'm task1 !");
        for _ in 1..1_000_000 {}
    }
}

#[unsafe(no_mangle)]
extern "C" fn task_2() -> ! {
    loop {
        info!("I'm task2 !");
        for _ in 1..100_000 {}
    }
}

fn config_systick(syst: &mut peripheral::SYST, freq: u32) {
    syst.set_clock_source(peripheral::syst::SystClkSource::Core);
    syst.set_reload(freq / 10);
    syst.clear_current();
    syst.enable_counter();
    syst.enable_interrupt();
}

#[hal::entry]
fn main() -> ! {
    let mut pac = hal::pac::Peripherals::take().unwrap();
    let core = cortex_m::Peripherals::take().unwrap();
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);
    let clocks = hal::clocks::init_clocks_and_plls(
        XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    unsafe {
        let sp1 = STACK_1.init(task_1);
        kernel::TASKS[0].sp = sp1;

        let sp2 = STACK_2.init(task_2);
        kernel::TASKS[1].sp = sp2;
    }

    let mut syst = core.SYST;
    let sys_freq = clocks.system_clock.freq().to_Hz();
    config_systick(&mut syst, sys_freq / 1000); // about 1500 Hz

    unsafe { core::arch::asm!("svc 0"); } // todo: cortex-m crate might implement this

    loop {
        cortex_m::asm::nop();
    }
}

#[inline(never)]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! { // inspired by panic_halt implementation
    error!("PANIC!");
    loop {
        atomic::compiler_fence(Ordering::SeqCst);
    }
}

#[exception]
unsafe fn HardFault(ef: &ExceptionFrame) -> ! {
    error!(
        "HardFault: r0={} r1={} r2={} r3={} r12={} lr={} pc={} xpsr={}",
        ef.r0(),
        ef.r1(),
        ef.r2(),
        ef.r3(),
        ef.r12(),
        ef.lr(),
        ef.pc(),
        ef.xpsr()
    );

    loop {
        cortex_m::asm::nop();
    }
}

// just a small description of the binary
#[unsafe(link_section = ".bi_entries")]
#[used]
pub static BINARY_ENTRIES: [binary_info::EntryAddr; 5] = [
    binary_info::rp_cargo_bin_name!(),
    binary_info::rp_cargo_version!(),
    binary_info::rp_program_description!(c"WHYOS"),
    binary_info::rp_cargo_homepage_url!(),
    binary_info::rp_program_build_attribute!(),
];