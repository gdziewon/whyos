#![no_std]
#![no_main]

mod whyos;

use whyos::Stack;

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

static STACK_1: Stack<4096> = Stack::new();
static STACK_2: Stack<4096> = Stack::new();
static STACK_3: Stack<4096> = Stack::new();

static SHARED_COUNTER: whyos::Mutex<u64> = whyos::Mutex::new(0);

extern "C" fn task_1() -> ! {
    loop {
        {
            let mut counter = SHARED_COUNTER.lock();

            *counter += 1;
            info!("task1: Count= {}", *counter);
        }

        whyos::sleep(500);
    }
}

extern "C" fn task_2() -> ! {
    loop {
        {
            let mut counter = SHARED_COUNTER.lock();

            *counter += 1;
            defmt::info!("task2: Count= {}", *counter);
        }

        whyos::sleep(600);
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

    whyos::add_task(&STACK_1, task_1, 255);
    whyos::add_task(&STACK_2, task_2, 1);
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