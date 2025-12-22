#![no_std]
#![no_main]

use whyos_demo::{board::Board, hal};
use defmt::info;

#[cfg(feature = "shell")]
use whyos_demo::shell;


#[hal::entry]
fn main() -> ! {
    let mut board = Board::init();

    #[cfg(feature = "shell")]
    shell::init_shell(board.uart);

    info!("Starting WhyOS...");
    unsafe { whyos::start(&mut board.syst, board.sys_freq / 1000); }
}