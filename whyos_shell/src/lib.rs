#![no_std]

use heapless::String;
use embedded_io::{Write, ErrorType};
use core::fmt;

const PROMPT: &str = "WhyOS> ";


pub struct Shell<'a, W> {
    input: &'a whyos::Queue<u8, 64>,
    output: W, // generic writer
    buffer: String<64>,
}

impl<'a, W> Shell<'a, W>
where
    W: ErrorType + Write
{
    pub fn new(input: &'a whyos::Queue<u8, 64>, output: W) -> Self {
        Self {
            input,
            output,
            buffer: String::new(),
        }
    }

    pub fn run(&mut self) {
        self.print_fmt(format_args!("\r\n{}", PROMPT));
        loop {
            let byte = self.input.receive(); // read byte

            let _ = self.output.write(&[byte]); // echo back

            match byte {
                b'\r' | b'\n' => { // Enter
                    self.print("\r\n");
                    self.process_command();
                    self.buffer.clear();
                    self.print(PROMPT);
                }
                b'\x08' | b'\x7F' => { // Backspace
                     if self.buffer.pop().is_some() {
                        self.print(" \x08"); // todo: this is simplified
                     }
                }
                c => {
                    if self.buffer.push(c as char).is_err() {
                        self.buffer.clear();
                        self.print(PROMPT);
                    }
                }
            }
        }
    }

    fn process_command(&mut self) {
        let cmd_string: String<64> = self.buffer.clone();
        let cmd = cmd_string.as_str().trim();

        match cmd {
            "help" | "h" | "?" => self.print("Available: help, ps, uptime\r\n"),

            "uptime" => {
                let ticks = whyos::uptime_ticks();

                self.print_fmt(format_args!("Uptime: {} ticks\r\n", ticks));
            }

            "ps" => {
                self.print(" ID | State     | Prio | Stack Size\r\n");
                self.print("----+-----------+------+------------\r\n");

                for tid in whyos::active_tasks() {
                    if let Ok(info) = whyos::task_info(tid) {
                        self.print_fmt(format_args!(
                            " {:>2} | {:<9} | {:>4} | {:>5}\r\n",
                            info.id,
                            info.state,
                            info.priority,
                            info.stack_size
                        ));
                    }
                }
            }

            "" => {},
            _ => {
                self.print_fmt(format_args!("Unknown command: {}\r\n", cmd));
            }
        }
    }

    fn print(&mut self, s: &str) {
        let _ = self.output.write_all(s.as_bytes());
        self.flush();
    }

    fn print_fmt(&mut self, args: fmt::Arguments) {
        let _ = self.output.write_fmt(args);
        self.flush();
    }

    fn flush(&mut self) {
        let _ = self.output.flush();
    }
}