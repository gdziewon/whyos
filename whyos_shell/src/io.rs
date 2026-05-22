use whyos::Mutex;
use core::fmt;
use embedded_io::{Write, ErrorType};

/// Callback invoked by the shell with raw output bytes.
pub type OutputFn = fn(&[u8]);

static STDOUT: Mutex<Option<OutputFn>> = Mutex::new(None);

pub(crate) fn set_stdout(printer: OutputFn) {
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

/// Writes formatted text to the shell output and appends a newline.
#[macro_export]
macro_rules! uprintln {
    ($($arg:tt)*) => {
        {
            $crate::io::_print_args(core::format_args!($($arg)*));
            $crate::io::_print_args(core::format_args!("\r\n"))
        }
    }
}

/// Writes formatted text to the shell output.
#[macro_export]
macro_rules! uprint {
    ($($arg:tt)*) => {
        $crate::io::_print_args(core::format_args!($($arg)*))
    }
}