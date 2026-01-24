#![no_std]

pub mod board;
pub mod rt;

#[cfg(feature = "shell")]
pub mod shell;

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
            return Err(concat!("Assertion failed: ", stringify!($cond)));
        }
    };
    ($cond:expr, $msg:expr) => {
        if !($cond) {
            return Err($msg);
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
        #[unsafe(no_mangle)]
        extern "C" fn test_wrapper(fn_addr: usize) -> ! {
            // transmute ptr to fn() -> TaskResult
            let test_fn: fn() -> $crate::TestResult = unsafe { core::mem::transmute(fn_addr) };

            {
                let mut status = TEST_STATUS.lock();
                match test_fn() {
                    Ok(_) => defmt::info!("OK"),
                    Err(msg) => {
                        defmt::error!("FAILED: {}", msg);
                        *status = false;
                    }
                }
            }

            TEST_DONE.signal(); // signal the runner

            whyos::exit();
        }

        #[unsafe(no_mangle)]
        extern "C" fn runner() -> ! {
            defmt::info!("TEST SUITE: {}\n", file!());

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
            let mut board = $crate::board::Board::init();

            whyos::TaskBuilder::new(runner)
                .priority(1)
                .stack_size(whyos::StackSize::MEDIUM)
                .name("Test Runner")
                .spawn()
                .unwrap();

            unsafe { whyos::start(&mut board.syst, board.sys_freq / 1000); }
        }
    };
}