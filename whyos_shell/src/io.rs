use whyos::Mutex;
use core::fmt;
use embedded_io::{Write, ErrorType};

pub type PrintFn = fn(&[u8]);

static STDOUT: Mutex<Option<PrintFn>> = Mutex::new(None);

pub(crate) fn set_stdout(printer: PrintFn) {
    *STDOUT.lock() = Some(printer);
}

#[derive(Clone, Copy)]
pub struct StdoutWriter;

impl ErrorType for StdoutWriter {
    type Error = core::convert::Infallible;
}

impl Write for StdoutWriter {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Self::Error> {
        if let Some(printer) = *STDOUT.lock() {
            printer(buf);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl fmt::Write for StdoutWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if let Some(printer) = *STDOUT.lock() {
            printer(s.as_bytes());
        }
        Ok(())
    }
}

#[doc(hidden)]
pub fn _print_args(args: fmt::Arguments) {
    let mut writer = StdoutWriter;
    let _ = fmt::Write::write_fmt(&mut writer, args);
}

#[macro_export]
macro_rules! uprintln {
    ($($arg:tt)*) => {
        {
            $crate::io::_print_args(core::format_args!($($arg)*));
            $crate::io::_print_args(core::format_args!("\r\n"))
        }
    }
}

#[macro_export]
macro_rules! uprint {
    ($($arg:tt)*) => {
        $crate::io::_print_args(core::format_args!($($arg)*))
    }
}