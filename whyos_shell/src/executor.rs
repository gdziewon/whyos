use crate::{Program, uprint, uprintln};
use crate::{Command, HELP_MSG};

pub fn execute<'a>(cmd: Command, mut programs: impl Iterator<Item = &'a Program> + 'a)
{
    match cmd {
        Command::Empty => (),
        Command::Help => uprint!("{}", HELP_MSG),
        Command::Name => uprintln!("{}", whyos::build_name()),
        Command::Reboot => {
            uprintln!("Rebooting system...");
            whyos::reboot()
        }

        Command::Uptime => {
            let ticks = whyos::uptime_ticks();
            uprintln!("{}", ticks);
        }

        Command::Ps => {
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

        Command::TaskInfo(handle) => {
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
                Err(_) => uprintln!("Error: Task not found"),
            }
        }

        Command::Suspend(handle) => {
            match handle.suspend() {
                Ok(_) => uprintln!("Task {} suspended", handle),
                Err(_) => uprintln!("Failed to suspend task {}", handle)
            }
        }

        Command::Resume(handle) => {
            match handle.resume() {
                Ok(_) => uprintln!("Task {} resumed", handle),
                Err(_) => uprintln!("Failed to resume task {}", handle)
            }
        }

        Command::Kill(handle) => {
            match handle.kill() {
                Ok(_) => uprintln!("Task {} killed", handle),
                Err(_) => uprintln!("Failed to kill task {}", handle),
            }
        }

        Command::Freq(new_freq) => {
            if let Some(freq) = new_freq {
                if freq == 0 {
                    uprintln!("Frequency cannot be 0");
                } else {
                    whyos::set_tick_freq(freq);
                    uprintln!("System frequency updated to {} Hz", freq);
                }
            } else {
                uprintln!("{} Hz", whyos::tick_freq());
            }
        }

        Command::Execute(name, arg_opt) => {
            if let Some(prog) = programs.find(|p| p.name == name) {
                let arg = arg_opt.unwrap_or(prog.default_arg);

                if let Err(e) = whyos::TaskBuilder::with_value(prog.entry, arg)
                    .name(prog.name)
                    .priority(prog.priority)
                    .stack_size(prog.stack_size)
                    .spawn() {
                        uprintln!("Couldn't execute '{}', error: {:?}", name, e);
                    }
            } else {
                uprintln!("Unknown program: '{}'", name);
            }
        }

        Command::List => {
            uprintln!(" Name           | Prio | Stack | Default | Description");
            uprintln!("────────────────+──────+───────+─────────+──────────────────────────");

            for prog in programs {
                uprintln!(
                    " {:<14} | {:<4} | {:<5} | {:<7} | {}",
                    prog.name,
                    prog.priority,
                    prog.stack_size.as_bytes(),
                    prog.default_arg,
                    prog.desc
                );
            }
            uprintln!("\r\nType 'execute <name> [arg]' to run a program.");
        }

        Command::Invalid(msg) => {
            uprintln!("{}", msg);
        }

        Command::Unknown(cmd_text) => {
            uprintln!("Unknown command: '{}'", cmd_text);
        }
    }
}