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
        "name" | "n" => Command::Name,
        "reboot" => Command::Reboot,
        "uptime" | "u" => Command::Uptime,
        "ps" | "p" => Command::Ps,
        "info" | "i" => parse_id(args, "Usage: info <id>", Command::TaskInfo),
        "suspend" | "s" => parse_id(args, "Usage: suspend <id>", Command::Suspend),
        "resume" | "r" => parse_id(args, "Usage: resume <id>", Command::Resume),
        "kill" | "k" => parse_id(args, "Usage: kill <id>", Command::Kill),
        "execute" | "e" => parse_exec(args),
        "list" | "l" => Command::List,
        _ => Command::Unknown(trimmed),
    }
}

fn parse_exec<'a>(args: &'a str) -> Command<'a> {
    let args = args.trim();
    if args.is_empty() {
        return Command::Invalid("Usage: exec <name> [arg]");
    }

    // if no space is found, arg_str is empty
    let (name, arg_str) = args.split_once(' ').unwrap_or((args, ""));
    let arg_str = arg_str.trim();

    let arg = if arg_str.is_empty() {
        None
    } else if let Ok(val) = arg_str.parse::<usize>() {
        Some(val)
    } else {
        return Command::Invalid("Error: Argument must be a number");
    };

    Command::Execute(name, arg)
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