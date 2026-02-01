use core::convert::Infallible;

use hal::pac::interrupt;
use embedded_hal_nb::serial::Read;
use embedded_io::{Write, ErrorType};

use crate::{hal, board::{Uart, UartRx, UartTx}};
use whyos::{Mutex, Queue, StackSize, TaskBuilder};
use whyos_shell::{Shell, Program};


static SHELL_RX_QUEUE: Queue<u8, 64> = Queue::new();

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
        $crate::shell::print(format_args!($($arg)*));
        $crate::shell::print(format_args!("\r\n"));
    }
}

pub fn init_shell(uart: Uart) {
    *(UART_RX.lock()) = Some(uart.rx);
    *(UART_TX.lock()) = Some(uart.tx);

    // enable NVIC interrupt for uart
    unsafe {
        cortex_m::peripheral::NVIC::unmask(hal::pac::Interrupt::UART0_IRQ);
    }

    TaskBuilder::new(shell_task)
        .priority(7)
        .stack_size(StackSize::LARGE)
        .name("shell")
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

extern "C" fn prog_counter(mut count: usize) {
    uprintln!("\r\n");
    while count > 0 {
        uprintln!("{}", count);
        count -= 1;
        whyos::sleep(1);
    }
}

static PROGRAMS: &[Program] = &[ // todo: add more programs
    Program {
        name: "counter",
        desc: "Counts down from N to 0",
        entry: prog_counter,
        default_arg: 10,
        priority: 2,
        stack_size: StackSize::SMALL
    }
];

#[unsafe(no_mangle)]
extern "C" fn shell_task() {
    let tx = SharedUart;

    let mut shell = Shell::new(&SHELL_RX_QUEUE, tx, PROGRAMS);
    shell.run();
}