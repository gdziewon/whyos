#![no_std]

mod parser;
mod executor;

use heapless::String;
use embedded_io::{Write, ErrorType};
use core::fmt::{self};

use whyos::{TaskId, Queue};

pub trait Writer: ErrorType + Write {}
impl<T: ErrorType + Write> Writer for T {}

const WELCOME_MSG: &str = "\r\nWhyOS Shell\r\n";
const PROMPT: &str = "Y-Oh!> ";
const BACKSPACE_SEQ: &str = "\x08 \x08"; // destructive backspace

enum Command<'a> {
    Help,
    Uptime,
    Ps,
    TaskInfo(TaskId),
    Suspend(TaskId),
    Resume(TaskId),
    Invalid(&'a str),
    Unknown(&'a str),
    Empty,
}

pub struct Shell<'a, W> {
    input: &'a Queue<u8, 64>,
    output: W,
    buffer: String<64>,
}

impl<'a, W: Writer> Shell<'a, W> {
    pub fn new(input: &'a Queue<u8, 64>, output: W) -> Self {
        Self {
            input,
            output,
            buffer: String::new(),
        }
    }

    pub fn run(&mut self) -> ! {
        print(&mut self.output, WELCOME_MSG);
        print(&mut self.output, PROMPT);

        loop {
            let byte = self.input.receive();

            match byte {
                // ENTER
                b'\r' | b'\n' => {
                    print(&mut self.output, "\r\n");

                    let text = self.buffer.as_str();

                    let cmd = parser::parse(text);

                    executor::execute(cmd, &mut self.output);

                    self.buffer.clear();

                    print(&mut self.output, PROMPT);
                }
                // BACKSPACE
                b'\x08' | 0x7F => {
                    if !self.buffer.is_empty() { // so user can't backspace the prompt
                        self.buffer.pop();
                        print(&mut self.output, BACKSPACE_SEQ);
                    }
                }
                // anything else
                c => {
                    if self.buffer.push(c as char).is_ok() {
                        // echo
                        let _ = self.output.write(&[c]);
                    }
                }
            }
        }
    }
}

pub(crate) fn print<W: Writer>(writer: &mut W, s: &str) {
    let _ = writer.write_all(s.as_bytes());
    let _ = writer.flush();
}

pub(crate) fn fprint<W: Writer>(writer: &mut W, args: fmt::Arguments) {
    let _ = writer.write_fmt(args);
    let _ = writer.flush();
}