#![no_std]

pub mod board;
pub mod img;

#[cfg(feature = "shell")]
pub mod shell;

#[cfg(feature = "shell")]
pub mod uart; // we might want to add uart logging  if i dont resolve rtt isues on riscv

pub use rp235x_hal as hal;

use defmt_rtt as _; // init rtt

use core::sync::atomic::{self, Ordering};

pub type TestResult = Result<(), &'static str>;

pub fn halt() -> ! {
    loop { atomic::compiler_fence(Ordering::SeqCst); }
}

#[macro_export]
macro_rules! check {
    ($cond:expr) => {
        if !($cond) {
            defmt::error!("Assertion failed: {} | {}:{}", stringify!($cond), file!(), line!());
            return Err("Assertion failed");
        }
    };
    ($cond:expr, $($arg:tt)+) => {
        if !($cond) {
            defmt::error!($($arg)+);
            return Err("Assertion failed");
        }
    };
}

// sort of test harness
// it uses priorities 1 and 2 for its helper tasks
#[macro_export]
macro_rules! harness {
    ( $( $test_fn:ident ),* $(,)? ) => {
        static TEST_DONE: whyos::Semaphore = whyos::Semaphore::new(0, 1); // semaphore for syncing with runner
        static TEST_STATUS: whyos::Mutex<bool> = whyos::Mutex::new(true); // holds status of currently finished testcase

        // runs test function and signals completion to runner
        extern "C" fn test_wrapper(fn_addr: usize) {
            // transmute ptr to fn() -> TaskResult
            let test_fn: fn() -> $crate::TestResult = unsafe { core::mem::transmute(fn_addr) };

            {
                match test_fn() {
                    Ok(_) => defmt::info!("OK"),
                    Err(msg) => {
                        defmt::error!("FAILED: {}", msg);
                        *TEST_STATUS.lock() = false;
                    }
                }
            }

            TEST_DONE.signal(); // signal the runner
        }

        extern "C" fn runner() {
            defmt::info!(" TEST SUITE: {}", file!());

            let mut any_failed = false;
            $(
                defmt::info!("{}:", stringify!($test_fn));
                *TEST_STATUS.lock() = true;

                let typed_fn: fn() -> $crate::TestResult = $test_fn; // for type safety, does nothing if function is of type 'fn() -> TestResult'
                let fn_addr = $test_fn as usize;

                whyos::TaskBuilder::with_value(test_wrapper, fn_addr)
                    .priority(2)
                    .stack_size(whyos::StackSize::DEFAULT)
                    .name(stringify!($test_fn))
                    .spawn()
                    .unwrap();

                TEST_DONE.wait(); // simulates "join()"
                let passed = *TEST_STATUS.lock();

                if !passed {
                    any_failed = true;
                }
            )*

            if any_failed {
                defmt::error!("=== TESTS FAILED ===");
            } else {
                defmt::info!("=== TESTS PASSED ===");
            }

            cortex_m_semihosting::debug::exit(cortex_m_semihosting::debug::EXIT_SUCCESS);
            $crate::halt();
        }

        #[$crate::hal::entry]
        fn main() -> ! {
            let board = $crate::board::Board::init();

            whyos::TaskBuilder::new(runner)
                .priority(1)
                .stack_size(whyos::StackSize::MEDIUM)
                .name("Test Runner")
                .spawn()
                .unwrap();

            whyos::start(whyos::Freq::ONE_KHZ);
        }
    };
}