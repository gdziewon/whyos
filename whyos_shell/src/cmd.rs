use crate::prog::{Program, PROGRAMS};
use crate::{uprintln, uprint};
use whyos::TaskHandle;

pub type CmdAction = fn(cmd: &Cmd, args: &str, user_programs: &[Program]);

pub struct Cmd {
    pub names: &'static [&'static str],
    pub usage: &'static str,
    pub desc: &'static str,
    pub action: CmdAction,
}

impl Cmd {
    /// Próbuje znaleźć komendę na podstawie wpisanego aliasu
    pub fn parse(name: &str) -> Option<&'static Cmd> {
        COMMANDS.iter().find(|cmd| cmd.names.contains(&name))
    }

    /// Wykonuje komendę, przekazując jej argumenty i listę programów
    pub fn run(&self, args: &str, user_programs: &[Program]) {
        (self.action)(self, args, user_programs)
    }
}

static COMMANDS: &[Cmd] = &[
    Cmd {
        names: &["help", "h", "?"],
        usage: "help|h|?",
        desc: "Show this help message",
        action: |_, _, _| {
            uprintln!("Commands:");
            for cmd in COMMANDS {
                uprintln!("  {:<26}  {}", cmd.usage, cmd.desc);
            }
        }
    },
    Cmd {
        names: &["clear", "cls"],
        usage: "clear|cls",
        desc: "Clear the terminal screen",
        action: |_, _, _| {
            const CLEAR_SEQ: &str = "\x1b[2J\x1b[3J\x1b[H";
            uprint!("{}", CLEAR_SEQ)
        }
    },
    Cmd {
        names: &["name", "n"],
        usage: "name|n",
        desc: "Print OS build name",
        action: |_, _, _| {
            uprintln!("{}", whyos::build_name());
        }
    },
    Cmd {
        names: &["reboot"],
        usage: "reboot",
        desc: "Reboot the system",
        action: |_, _, _| {
            uprintln!("Rebooting system...");
            whyos::reboot();
        }
    },
    Cmd {
        names: &["uptime", "u"],
        usage: "uptime",
        desc: "Show system uptime in ticks",
        action: |_, _, _| {
            let ticks = whyos::uptime_ticks();
            uprintln!("{} ticks", ticks);
        }
    },
    Cmd {
        names: &["ps"],
        usage: "ps",
        desc: "List all tasks",
        action: |_, _, _| {
            uprintln!(" ID | State     | Stack(Peak/Total) | Name");
            uprintln!("────+───────────+───────────────────+──────────────");

            for handle in whyos::allocated() {
                if let Ok(info) = handle.info() {
                    let name = info.name.unwrap_or("-");

                    let pct = (info.max_stack_usage * 100) / info.stack_size;

                    uprintln!(" {:>4} | {:<9} | {:>4} / {:<4} ({:>2}%) | {}",
                        info.handle.as_u32(),
                        info.state,
                        info.max_stack_usage,
                        info.stack_size,
                        pct,
                        name
                    );
                }
            }
        }
    },
    Cmd {
        names: &["info", "i"],
        usage: "info|i <id>",
        desc: "Show detailed task information",
        action: |cmd, args, _| {
            if let Some(handle) = parse_id(args) {
                match handle.info() {
                    Ok(info) => {
                        let stack_top = info.stack_base + info.stack_size;
                        let current_usage = stack_top.saturating_sub(info.current_sp);

                        uprintln!(
                            "──────────────────────────────────────────\r\n\
                            Task ID:      {}\r\n\
                            Name:         {}\r\n\
                            State:        {}\r\n\
                            Priority:     {}\r\n\
                            ------------------------------------------\r\n\
                            Stack Base:   0x{:08x}\r\n\
                            Stack Ptr:    0x{:08x}\r\n\
                            Stack Size:   {} bytes\r\n\
                            Current Use:  {} bytes\r\n\
                            Peak Usage:   {} bytes ({}%)\r\n\
                            ──────────────────────────────────────────",
                            info.handle,
                            info.name.unwrap_or("<unnamed>"),
                            info.state,
                            info.priority,
                            info.stack_base,
                            info.current_sp,
                            info.stack_size,
                            current_usage,
                            info.max_stack_usage,
                            (info.max_stack_usage * 100) / info.stack_size
                        );
                    }
                    Err(e) => uprintln!("Error: {}", e),
                }
            } else {
                uprintln!("Usage: {}", cmd.usage)
            }
        }
    },
    Cmd {
        names: &["suspend", "s"],
        usage: "suspend|s <id>",
        desc: "Suspend a task",
        action: |cmd, args, _| {
            if let Some(handle) = parse_id(args) {
                match handle.suspend() {
                    Ok(_) => uprintln!("Task {} suspended", handle),
                    Err(_) => uprintln!("Error: Failed to suspend task {}", handle)
                }
            } else {
                uprintln!("Usage: {}", cmd.usage);
            }
        }
    },
    Cmd {
        names: &["resume", "r"],
        usage: "resume|r <id>",
        desc: "Resume a suspended task",
        action: |cmd, args, _| {
            if let Some(handle) = parse_id(args) {
                match handle.resume() {
                    Ok(_) => uprintln!("Task {} resumed", handle),
                    Err(e) => uprintln!("Error: {}", e)
                }
            } else {
                uprintln!("Usage: {}", cmd.usage);
            }
        }
    },
    Cmd {
        names: &["kill", "k"],
        usage: "kill|k <id>",
        desc: "Kill a task (unsafe)",
        action: |cmd, args, _| {
            if let Some(handle) = parse_id(args) {
                match handle.kill() {
                    Ok(_) => uprintln!("Task {} killed", handle),
                    Err(e) => uprintln!("Error: {}", e)
                }
            } else {
                uprintln!("Usage: {}", cmd.usage);
            }
        }
    },
    Cmd {
        names: &["freq"],
        usage: "freq [hz]",
        desc: "Get or set system tick frequency",
        action: |cmd, args, _| {
            let args = args.trim();
            if args.is_empty() {
                uprintln!("{} Hz", whyos::tick_freq());
            } else if let Ok(freq) = args.parse::<u32>() {
                if let Some(freq) = whyos::Freq::from_hz(freq) {
                    whyos::set_tick_freq(freq);
                    uprintln!("System frequency updated to {} Hz", freq.as_hz());
                } else {
                    uprintln!("Error: Frequency cannot be 0");
                }
            } else {
                uprintln!("Usage: {}", cmd.usage);
            }
        }
    },
    Cmd {
        names: &["list"],
        usage: "list",
        desc: "List available programs",
        action: |_, _, user_progs| {
            uprintln!(" Name           | Prio | Stack | Default | Description");
            uprintln!("────────────────+──────+───────+─────────+──────────────────────────");

            let all_progs = PROGRAMS.iter().chain(user_progs.iter());
            for prog in all_progs {
                uprintln!(
                    " {:<14} | {:<4} | {:<5} | {:<7} | {}",
                    prog.name,
                    prog.priority,
                    prog.stack_size.as_bytes(),
                    prog.default_arg,
                    prog.desc
                );
            }
        }
    },
    Cmd {
        names: &["execute", "e"],
        usage: "execute|e <name> [num_arg]",
        desc: "Execute a program",
        action: |cmd, args, user_progs| {
            let args = args.trim();
            let (name, arg_str) = args.split_once(' ').unwrap_or((args, ""));
            let arg_str = arg_str.trim();

            if name.is_empty() {
                uprintln!("Usage: {}", cmd.usage);
                return;
            }

            let parsed_arg = if arg_str.is_empty() {
                None
            } else if let Ok(val) = arg_str.parse::<usize>() {
                Some(val)
            } else {
                uprintln!("Usage: {}", cmd.usage);
                return;
            };

            let all_progs = PROGRAMS.iter().chain(user_progs.iter());
            for prog in all_progs {
                if prog.name == name {
                    let arg = parsed_arg.unwrap_or(prog.default_arg);
                    if let Err(e) = whyos::TaskBuilder::with_value(prog.entry, arg)
                        .name(prog.name)
                        .priority(prog.priority)
                        .stack_size(prog.stack_size)
                        .spawn()
                    {
                        uprintln!("Couldn't execute '{}', error: {}", name, e);
                    } else {
                        uprintln!("Spawned task '{}' (arg: {})", name, arg);
                    }
                    return;
                }
            }
            uprintln!("Unknown program: '{}'. Type 'list' to see available.", name);
        }
    },
    Cmd {
        names: &["peek"],
        usage: "peek <hex_addr>",
        desc: "Read a 32-bit value from memory (might HardFault)",
        action: |cmd, args, _| {
            if let Some(addr) = parse_hex(args) {
                if addr % 4 != 0 {
                    uprintln!("Error: Address must be 4-byte aligned!");
                    return;
                }
                let val = unsafe { core::ptr::read_volatile(addr as *const u32) };
                uprintln!("Value at 0x{:08X} = 0x{:08X}", addr, val);
            } else {
                uprintln!("Usage: {}", cmd.usage);
            }
        }
    },
    Cmd {
        names: &["poke"],
        usage: "poke <hex_addr> <hex_val>",
        desc: "Write a 32-bit value to memory (might HardFault)",
        action: |cmd, args, _| {
            let (addr_str, val_str) = args.trim().split_once(' ').unwrap_or(("", ""));

            if let (Some(addr), Some(val)) = (parse_hex(addr_str), parse_hex(val_str)) {
                if addr % 4 != 0 {
                    uprintln!("Error: Address must be 4-byte aligned!");
                    return;
                }
                unsafe { core::ptr::write_volatile(addr as *mut u32, val as u32) };
                uprintln!("Written 0x{:08X} to 0x{:08X}", val as u32, addr);
            } else {
                uprintln!("Usage: {}", cmd.usage);
            }
        }
    },
];

fn parse_id(args: &str) -> Option<TaskHandle> {
    let args = args.trim();
    if args.is_empty() { return None; }

    args.parse::<u32>().ok().and_then(|id| TaskHandle::from_u32(id))
}

fn parse_hex(args: &str) -> Option<usize> {
    let args = args.trim().strip_prefix("0x").unwrap_or(args.trim());
    if args.is_empty() { return None; }

    usize::from_str_radix(args, 16).ok()
}