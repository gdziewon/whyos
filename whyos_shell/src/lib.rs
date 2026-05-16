#![no_std]

pub use embedded_io;

mod io;
mod prog;
mod cmd;
pub use prog::Program;

use heapless::String;
use embedded_io::{Write, ErrorType};

use whyos::{Queue, TaskHandle};

use crate::{cmd::{Cmd, Env}, io::PrintFn};

pub trait Writer: ErrorType + Write {}
impl<T: ErrorType + Write> Writer for T {}

const BUFFER_SIZE: usize = 64;

const WELCOME_MSG: &str = "WhyOS Shell";
const PROMPT: &str = "Y-Oh!> ";
const BACKSPACE_SEQ: &str = "\x08 \x08"; // destructive backspace
const CTRL_C: u8 = 0x03;
const SHOW_CURSOR: &str = "\x1b[?25h";

pub struct Shell<'a> {
    input: &'a Queue<u8, BUFFER_SIZE>,
    buffer: String<BUFFER_SIZE>,
    user_programs: &'a [Program],
    last_task: Option<TaskHandle>
}

impl<'a> Shell<'a> { // todo: validate no duplicate names on user_progs
    pub fn new(input: &'a Queue<u8, 64>, write_fn: PrintFn, user_programs: &'a [Program]) -> Self {
        io::set_stdout(write_fn);
        Self {
            input,
            buffer: String::new(),
            user_programs,
            last_task: None
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

                    let text = self.buffer.as_str().trim();
                    if !text.is_empty() {
                        let (cmd_name, args) = text.split_once(' ').unwrap_or((text, ""));

                        if let Some(cmd) = Cmd::parse(cmd_name) {
                            let mut env = Env { user_programs: &self.user_programs, last_task: &mut self.last_task };
                            cmd.run(args, &mut env);
                        } else {
                            uprintln!("Unknown command: '{}'. Type 'help' for a list of commands.", cmd_name);
                        }
                    }

                    self.buffer.clear();
                    uprint!("{}", PROMPT);
                },
                CTRL_C => {
                    uprintln!("^C{}", SHOW_CURSOR);

                    if let Some(handle) = self.last_task.take() {
                        let _ = handle.kill();
                    }

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