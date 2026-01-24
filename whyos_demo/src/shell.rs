use crate::{hal, board::{Uart, UartRx, UartTx}};
use whyos::{Mutex, Queue, StackSize, TaskBuilder};

use hal::pac::interrupt;
use embedded_hal_nb::serial::Read;

static SHELL_RX_QUEUE: Queue<u8, 64> = Queue::new();

static UART_RX: Mutex<Option<UartRx>> = Mutex::new(None);
static UART_TX: Mutex<Option<UartTx>> = Mutex::new(None);

pub fn init_shell(uart: Uart) {
    *(UART_RX.lock()) = Some(uart.rx);
    *(UART_TX.lock()) = Some(uart.tx);

    // enable NVIC interrupt for uart
    unsafe {
        cortex_m::peripheral::NVIC::unmask(hal::pac::Interrupt::UART0_IRQ);
    }

    TaskBuilder::new(shell_task)
        .priority(2)
        .stack_size(StackSize::MEDIUM)
        .spawn()
        .unwrap();
}

#[interrupt]
fn UART0_IRQ() {
    let mut guard = UART_RX.lock();

    if let Some(rx) = guard.as_mut() {
        loop {
            match rx.read() {
                Ok(byte) => {
                    // data! push to queu
                    let _ = SHELL_RX_QUEUE.try_send(byte);
                }
                Err(nb::Error::WouldBlock) => {
                    // no more data
                    break;
                }
                Err(_e) => {
                    break; // hw error
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
extern "C" fn shell_task() -> ! {
    let tx: UartTx;
    {
        let mut tx_guard = UART_TX.lock();
        tx = tx_guard.take().expect("UART TX not initialized"); // take out of mutex to avoid constant locking
    }

    let mut shell = whyos_shell::Shell::new(&SHELL_RX_QUEUE, tx);
    shell.run();
}