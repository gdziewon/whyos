#![no_std]
#![no_main]

use whyos::{Semaphore};

use core::panic::PanicInfo;
use core::sync::atomic::{self, Ordering};

use rp235x_hal::{self as hal, Clock as _};
use hal::{binary_info, block::ImageDef};
use cortex_m_rt::{exception, ExceptionFrame};
use defmt::{info, error};
use defmt_rtt as _;

#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: ImageDef = ImageDef::secure_exe();

const XTAL_FREQ_HZ: u32 = 12_000_000u32; // 12 MHz

static SIGNAL: Semaphore = Semaphore::new(0, 1);

extern "C" fn task_1() -> ! {
    loop {
        info!("WAITER: waiting.");

        SIGNAL.wait();

        info!("WAITER: GOT SIGNAL!");
        whyos::sleep(200);
    }
}

extern "C" fn task_2() -> ! {
    loop {
        info!("SIGNALER: sleep");
        whyos::sleep(1000);

        info!("SIGNALER: signaling!");
        SIGNAL.signal();
    }
}

extern "C" fn task_3() -> ! {
    loop {
        info!("I'm task3 !");
        whyos::sleep(700 * 5);
    }
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

    whyos::add_task(task_1, 1, 4096);
    whyos::add_task(task_2, 2, 4096);
    //whyos::add_task(&STACK_3, task_3, 3);

    let mut syst = core.SYST;
    let sys_freq = clocks.system_clock.freq().to_Hz();

    unsafe { whyos::start(&mut syst, sys_freq / 1000); }
}

#[inline(never)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! { // inspired by panic_halt implementation
    error!("PANIC! {}", info);
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