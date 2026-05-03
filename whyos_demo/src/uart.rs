use core::convert::Infallible;

use embedded_hal_nb::serial::Read;

use crate::{hal, board::{Uart, UartRx, UartTx}};
use whyos::{Mutex, Queue};
use whyos_shell::embedded_io::{Write, ErrorType};


pub static SHELL_RX_QUEUE: Queue<u8, 64> = Queue::new();

static UART_RX: Mutex<Option<UartRx>> = Mutex::new(None);
static UART_TX: Mutex<Option<UartTx>> = Mutex::new(None);

#[derive(Clone, Copy)]
pub struct SharedUart;

impl ErrorType for SharedUart {
    type Error = Infallible;
}

impl Write for SharedUart {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        let mut guard = UART_TX.lock();
        if let Some(tx) = guard.as_mut() {
            tx.write(buf)
        } else {
            Ok(0)
        }
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        let mut guard = UART_TX.lock();
        if let Some(tx) = guard.as_mut() {
            tx.flush()
        } else {
            Ok(())
        }
    }
}

pub fn print(args: core::fmt::Arguments) {
    let mut writer = SharedUart;
    let _ = writer.write_fmt(args);
}

#[macro_export]
macro_rules! uprintln {
    ($($arg:tt)*) => {
        $crate::uart::print(format_args!($($arg)*));
        $crate::uart::print(format_args!("\r\n"));
    }
}

#[allow(non_snake_case)]
#[unsafe(no_mangle)]
fn UART0_IRQ() {
    uart_handler();
}

#[inline]
fn uart_handler() {
    if let Some(mut guard) = UART_RX.try_lock() {
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
}

pub fn init_uart(uart: Uart) {
    if (*UART_RX.lock()).is_some() || (*UART_TX.lock()).is_some() {
        return;
    }

    *UART_RX.lock() = Some(uart.rx);
    *UART_TX.lock() = Some(uart.tx);

    unsafe {
        hal::arch::interrupt_unmask(hal::pac::Interrupt::UART0_IRQ);
    }
}