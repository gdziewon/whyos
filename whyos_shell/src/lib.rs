//! *WhyOS Shell*, used for rendering terminal output, running shell commands and executing
//! programs.
//! It's a great tool for exploring [`WhyOS`](https://crates.io/crates/whyos) functionality.
//!
//! It requires setting up and providing input `Queue`, as well as output function, typically used with UART.
//!
//! # What can it do?
//!
//! Builtin commands:
//!
//! ```text
//!  help|h|?                    Show this help message
//!  clear|cls                   Clear the terminal screen
//!  name|n                      Print OS build name
//!  reboot                      Reboot the system
//!  uptime                      Show system uptime in ticks
//!  ps                          List all tasks
//!  info|i <id>                 Show detailed task information
//!  suspend|s <id>              Suspend a task
//!  resume|r <id>               Resume a suspended task
//!  kill|k <id>                 Kill a task (unsafe)
//!  freq [hz]                   Get or set system tick frequency
//!  list                        List available programs
//!  execute|e <name> [num_arg]  Execute a program
//!  peek <hex_addr>             Read a 32-bit value from memory (might HardFault)
//!  poke <hex_addr> <hex_val>   Write a 32-bit value to memory (might HardFault)
//! ```
//!
//! It also comes in with couple of example __programs__.
//!
//! ```text
//!  Name           | Prio | Stack | Default | Description
//! ────────────────+──────+───────+─────────+──────────────────────────
//!  cnt            | 2    | 1024  | 10      | Counts down from N to 0
//!  fib            | 2    | 1024  | 10      | Calculates N-th Fibonacci number (up to 186th)
//!  tim            | 3    | 1024  | 10000   | Sets a timer for N ticks
//!  panic          | 1    | 1024  | 0       | Triggers panic
//!  hard           | 1    | 1024  | 0       | Triggers Hard Fault
//!  magic          | 3    | 1024  | 0       | Wait for someone to poke memory to stop it
//!  top            | 2    | 2048  | 500     | Live task view with N refresh ticks
//!  eater          | 3    | 8192  | 90      | Slowly eats its own stack up to ~N% - setting 95 or more will cause HardFault
//!
//! ```

#![no_std]

mod io;
mod prog;
mod cmd;

pub use embedded_io;
pub use prog::Program;
pub use io::OutputFn;

use heapless::String;

use whyos::{Queue, TaskHandle};

use crate::cmd::{Cmd, Env};


/// Number of `u8` characters that Shell's buffer can hold.
pub const BUFFER_SIZE: usize = 64;

const WELCOME_MSG: &str = "WhyOS Shell";
const PROMPT: &str = "Y-Oh!> ";
const BACKSPACE_SEQ: &str = "\x08 \x08"; // destructive backspace
const CTRL_C: u8 = 0x03;
const SHOW_CURSOR: &str = "\x1b[?25h";

/// Interactive WhyOS shell loop.
pub struct Shell<'a> {
    input: &'a Queue<u8, BUFFER_SIZE>,
    buffer: String<BUFFER_SIZE>,
    user_programs: &'a [Program],
    last_task: Option<TaskHandle>
}

impl<'a> Shell<'a> { // todo: validate no duplicate names on user_progs
    /// Creates a new `Shell`.
    ///
    /// Output function is then used as a callback by `uprint!` and `uprintln!` macros.
    /// Provided user programs will be listed and executable along with builtin, example programs.
    pub fn new(input: &'a Queue<u8, BUFFER_SIZE>, output_fn: OutputFn, user_programs: &'a [Program]) -> Self {
        io::set_stdout(output_fn);
        Self {
            input,
            buffer: String::new(),
            user_programs,
            last_task: None
        }
    }

    /// Runs the shell forever until the system stops the task.
    ///
    /// Blocks waiting for data to be available in input queue.
    ///
    /// Supports:
    /// * `ENTER` - attempts to parse and execute command stored inside the buffer.
    /// * `CTRL+C` - attempts to kill the last executed program.
    /// * `BACKSPACE` - removes the last character in buffer if available.
    /// * `_` - pushes the character into the buffer, if not full, check `BUFFER_SIZE`.
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
                            let mut env = Env { user_programs: self.user_programs, last_task: &mut self.last_task };
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