use crate::Command;

pub fn parse(text: &str) -> Command<'_> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return Command::Empty;
        }

        let (cmd, args) = trimmed
            .split_once(' ')
            .unwrap_or((trimmed, ""));

        match cmd {
            "help" | "h" | "?" => Command::Help,
            "uptime" | "u" => Command::Uptime,
            "ps" | "p" => Command::Ps,
            "info" | "i" => parse_id(args, "Usage: info <id>", Command::TaskInfo),
            "suspend" | "s" => parse_id(args, "Usage: suspend <id>", Command::Suspend),
            "resume" | "r" => parse_id(args, "Usage: resume <id>", Command::Resume),
            _ => Command::Unknown(trimmed),
        }
    }

fn parse_id<F>(args: &str, usage: &'static str, constructor: F) -> Command<'static>
where
    F: FnOnce(whyos::TaskId) -> Command<'static>
{
    let args = args.trim();
    if args.is_empty() {
        return Command::Invalid(usage);
    }

    if let Ok(id) = args.parse::<usize>() {
        if let Ok(tid) = whyos::TaskId::new(id) {
            return constructor(tid);
        } else {
            return Command::Invalid("Error: Invalid Task ID");
        }
    }

    Command::Invalid("Error: ID must be a number")
}