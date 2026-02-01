use crate::{fprint, print};
use crate::{Writer, Command};

const HELP_MSG: &str = "Available: help, ps, uptime, info <id>, suspend <id>, resume <id>\r\n";

pub fn execute<'a, W: Writer>(cmd: Command, writer: &mut W)
{
    match cmd {
        Command::Empty => (),
        Command::Help => print(writer, HELP_MSG),
        Command::Uptime => {
            let ticks = whyos::uptime_ticks();
            fprint(writer, format_args!("{}\r\n", ticks));
        }

        Command::Ps => {
            print(writer, " ID | State     | Name\r\n");
            print(writer, "────+───────────+──────────\r\n");

            for tid in whyos::active_tasks() {
                if let Ok(info) = whyos::task_info(tid) {
                    let name = info.name.unwrap_or("-");
                    fprint(writer, format_args!(
                        " {:>2} | {:<9} | {}\r\n",
                        info.id, info.state, name
                    ));
                }
            }
        }

        Command::TaskInfo(tid) => {
            match whyos::task_info(tid) {
                Ok(info) => {
                    fprint(writer, format_args!(
                        "────────────────────────\r\n\
                            Task ID:      {}\r\n\
                            Name:         {}\r\n\
                            State:        {}\r\n\
                            Priority:     {}\r\n\
                            Stack Size:   {} bytes\r\n\
                         ────────────────────────\r\n",
                        info.id,
                        info.name.unwrap_or("<unnamed>"),
                        info.state,
                        info.priority,
                        info.stack_size
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
        Command::Invalid(msg) => {
            fprint(writer, format_args!("{}\r\n", msg));
        }
        Command::Unknown(cmd_text) => {
            fprint(writer, format_args!("Unknown command: '{}'\r\n", cmd_text));
        }
    }
}