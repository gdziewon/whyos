use crate::{Program, fprint, print};
use crate::{Writer, Command, HELP_MSG};

pub fn execute<W: Writer>(cmd: Command, programs: &[Program], writer: &mut W)
{
    match cmd {
        Command::Empty => (),
        Command::Help => print(writer, HELP_MSG),
        Command::Name => fprint(writer, format_args!("{}\r\n", whyos::build_name())),
        Command::Reboot => {
            print(writer, "Rebooting system...\r\n");
            whyos::reboot()
        }

        Command::Uptime => {
            let ticks = whyos::uptime_ticks();
            fprint(writer, format_args!("{}\r\n", ticks));
        }

        Command::Ps => {
            print(writer, " ID | State     | Stack(Peak/Total) | Name\r\n");
            print(writer, "────+───────────+───────────────────+──────────────\r\n");

            for tid in whyos::allocated_tasks() {
                if let Ok(info) = whyos::task_info(tid) {
                    let name = info.name.unwrap_or("-");

                    // todo: guard again div by 0?
                    let pct = (info.max_stack_usage * 100) / info.stack_size;

                    fprint(writer, format_args!(
                        " {:>2} | {:<9} | {:>4} / {:<4} ({:>2}%) | {}\r\n",
                        info.tid.id(),
                        info.state,
                        info.max_stack_usage,
                        info.stack_size,
                        pct,
                        name
                    ));
                }
            }
        }

        Command::TaskInfo(tid) => {
            match whyos::task_info(tid) {
                Ok(info) => {
                    let stack_top = info.stack_base + info.stack_size;
                    let current_usage = stack_top.saturating_sub(info.current_sp);

                    fprint(writer, format_args!(
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
                        ──────────────────────────────────────────\r\n",
                        info.tid.id(),
                        info.name.unwrap_or("<unnamed>"),
                        info.state,
                        info.priority,
                        info.stack_base,
                        info.current_sp,
                        info.stack_size,
                        current_usage,
                        info.max_stack_usage,
                        (info.max_stack_usage * 100) / info.stack_size
                    ));
                }
                Err(_) => print(writer, "Error: Task not found\r\n"),
            }
        }

        Command::Suspend(tid) => {
            match whyos::suspend(tid) {
                Ok(_) => fprint(writer, format_args!("Task {} suspended\r\n", tid.id())),
                Err(_) => fprint(writer, format_args!("Failed to suspend task {}\r\n", tid.id()))
            }
        }

        Command::Resume(tid) => {
            match whyos::resume(tid) {
                Ok(_) => fprint(writer, format_args!("Task {} resumed\r\n", tid.id())),
                Err(_) => fprint(writer, format_args!("Failed to resume task {}\r\n", tid.id()))
            }
        }

        Command::Kill(tid) => {
            match unsafe { whyos::kill(tid) } {
                Ok(_) => fprint(writer, format_args!("Task {} killed\r\n", tid.id())),
                Err(_) => fprint(writer, format_args!("Failed to kill task {}\r\n", tid.id())),
            }
        }

        Command::Execute(name, arg_opt) => {
            if let Some(prog) = programs.iter().find(|p| p.name == name) {
                let arg = arg_opt.unwrap_or(prog.default_arg);

                if let Err(e) = whyos::TaskBuilder::with_value(prog.entry, arg)
                    .name(prog.name)
                    .priority(prog.priority)
                    .stack_size(prog.stack_size)
                    .spawn() {
                        fprint(writer, format_args!("Couldn't execute '{}', error: {:?}", name, e));
                    }
            } else {
                fprint(writer, format_args!("Unknown program {}", name));
            }
        }

        Command::List => {
            print(writer, " Name           | Prio | Stack | Default | Description\r\n");
            print(writer, "────────────────+──────+───────+─────────+──────────────────────────\r\n");

            for prog in programs {
                fprint(writer, format_args!(
                    " {:<14} | {:<4} | {:<5} | {:<7} | {}\r\n",
                    prog.name,
                    prog.priority,
                    prog.stack_size.as_bytes(),
                    prog.default_arg,
                    prog.desc
                ));
            }
            print(writer, "\r\nType 'execute <name> [arg]' to run a program.\r\n");
        }

        Command::Invalid(msg) => {
            fprint(writer, format_args!("{}\r\n", msg));
        }

        Command::Unknown(cmd_text) => {
            fprint(writer, format_args!("Unknown command: '{}'\r\n", cmd_text));
        }
    }
}