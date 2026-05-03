#![no_std]

pub use embedded_io;

mod parser;
mod executor;

use heapless::String;
use embedded_io::{Write, ErrorType};
use core::fmt::{self};

use whyos::{Queue, StackSize, TaskId, TaskRoutineArg};

pub trait Writer: ErrorType + Write {}
impl<T: ErrorType + Write> Writer for T {}

const HELP_MSG: &str = "\
Commands:\r
  help|h|?                    Show this help message\r
  ps|p                        List all tasks\r
  uptime|u                    Show system uptime in ticks\r
  info|i <id>                 Show detailed task information\r
  suspend|s <id>              Suspend a task\r
  resume|r <id>               Resume a suspended task\r
  kill|k <id>                 Kill a task (unsafe)\r
  list|l                      List available programs\r
  execute|e <name> [arg]      Execute a program\r
  reboot                      Reboot the system\r
";
const WELCOME_MSG: &str = "\r\nWhyOS Shell\r\n";
const PROMPT: &str = "Y-Oh!> ";
const BACKSPACE_SEQ: &str = "\x08 \x08"; // destructive backspace

pub struct Program {
    pub name: &'static str,
    pub desc: &'static str,
    pub entry: TaskRoutineArg<usize>,
    pub default_arg: usize,
    pub priority: u8,
    pub stack_size: StackSize,
}

enum Command<'a> {
    Help,
    Reboot,
    Uptime,
    Ps,
    TaskInfo(TaskId),
    Suspend(TaskId),
    Resume(TaskId),
    Kill(TaskId),
    Execute(&'a str, Option<usize>),
    List,
    Invalid(&'a str),
    Unknown(&'a str),
    Empty,
}

pub struct Shell<'a, W> {
    input: &'a Queue<u8, 64>,
    output: W,
    buffer: String<64>,
    programs: &'a [Program]
}

impl<'a, W: Writer> Shell<'a, W> { // todo: validate no duplicate names
    pub fn new(input: &'a Queue<u8, 64>, output: W, programs: &'a [Program]) -> Self {
        Self {
            input,
            output,
            buffer: String::new(),
            programs
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

                    executor::execute(cmd, self.programs, &mut self.output);

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