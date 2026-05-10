#![no_std]

pub use embedded_io;

mod parser;
mod executor;
mod io;
mod prog;
pub use prog::Program;

use heapless::String;
use embedded_io::{Write, ErrorType};

use whyos::{Queue, TaskHandle};

use crate::{io::PrintFn, prog::PROGRAMS};

pub trait Writer: ErrorType + Write {}
impl<T: ErrorType + Write> Writer for T {}

const HELP_MSG: &str = "\
Commands:\r
  help|h|?                    Show this help message\r
  name|n                      Print build name\r
  ps|p                        List all tasks\r
  uptime|u                    Show system uptime in ticks\r
  info|i <id>                 Show detailed task information\r
  suspend|s <id>              Suspend a task\r
  resume|r <id>               Resume a suspended task\r
  kill|k <id>                 Kill a task (unsafe)\r
  freq [hz]                   Get or set system tick frequency\r
  list|l                      List available programs\r
  execute|e <name> [arg]      Execute a program\r
  reboot                      Reboot the system\r
";
const WELCOME_MSG: &str = "WhyOS Shell";
const PROMPT: &str = "Y-Oh!> ";
const BACKSPACE_SEQ: &str = "\x08 \x08"; // destructive backspace



enum Command<'a> {
    Help,
    Name,
    Reboot,
    Uptime,
    Ps,
    TaskInfo(TaskHandle),
    Suspend(TaskHandle),
    Resume(TaskHandle),
    Kill(TaskHandle),
    Execute(&'a str, Option<usize>),
    List,
    Freq(Option<u32>),
    Invalid(&'a str),
    Unknown(&'a str),
    Empty,
}

pub struct Shell<'a> {
    input: &'a Queue<u8, 64>,
    buffer: String<64>,
    user_programs: &'a [Program]
}

impl<'a> Shell<'a> { // todo: validate no duplicate names
    pub fn new(input: &'a Queue<u8, 64>, write_fn: PrintFn, user_programs: &'a [Program]) -> Self {
        io::set_stdout(write_fn);
        Self {
            input,
            buffer: String::new(),
            user_programs
        }
    }

    pub fn run(&mut self) -> ! {
        uprintln!("");
        uprintln!("{}", WELCOME_MSG);
        uprint!("{}", PROMPT);

        loop {
            let byte = self.input.receive();

            match byte {
                // ENTER
                b'\r' | b'\n' => {
                    uprintln!("");

                    let text = self.buffer.as_str();

                    let cmd = parser::parse(text);

                    executor::execute(cmd, PROGRAMS.iter().chain(self.user_programs));

                    self.buffer.clear();

                    uprint!("{}", PROMPT);
                }
                // BACKSPACE
                b'\x08' | 0x7F => {
                    if !self.buffer.is_empty() { // so user can't backspace the prompt
                        self.buffer.pop();
                        uprint!("{}", BACKSPACE_SEQ);
                    }
                }
                // anything else
                c => {
                    if self.buffer.push(c as char).is_ok() {
                        // echo
                       uprint!("{}", c as char);
                    }
                }
            }
        }
    }
}