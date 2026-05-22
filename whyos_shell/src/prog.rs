use whyos::{StackSize, TaskRoutineArg};

use crate::{uprint, uprintln};

/// Describes a runnable program that can be launched from the shell.
pub struct Program {
    /// Program name used in shell commands.
    pub name: &'static str,
    /// Short human-readable description shown by `list`.
    pub desc: &'static str,
    /// Task entry function invoked when the program is executed.
    pub entry: TaskRoutineArg<usize>,
    /// Default argument passed when the user omits one.
    pub default_arg: usize,
    /// Task priority used when spawning the program.
    pub priority: u8,
    /// Stack size requested for the task.
    pub stack_size: StackSize,
}

extern "C" fn prog_fib(mut num: usize) {
    let mut a: u128 = 0;
    let mut b: u128 = 1;
    while num > 0 {
        (a, b) = (b, match a.checked_add(b) {
            Some(v) => v,
            None => {
                uprintln!("Would overflow u128");
                return;
            }
        });
        num -= 1;
    }

    uprintln!("{}", a); // this printing u128 adds 2KiB to binary btw
}

extern "C" fn prog_counter(mut count: usize) {
    uprintln!("\r\n");
    while count > 0 {
        uprintln!("{}", count);
        count -= 1;
        whyos::sleep(1);
    }
}

extern "C" fn prog_timer(mut ticks: usize) {
    let tid = whyos::my_handle().as_u32();
    while ticks > 0 {
        ticks -= 1;
        whyos::sleep(1);
    }
    uprintln!("TIMER{} DONE", tid);
}

extern "C" fn prog_panic(_: usize) {
    uprintln!("AGHHH!");
    panic!()
}

extern "C" fn prog_hardfault(_: usize) {
    uprintln!("hard.");
    whyos::sleep(10);

    unsafe {
        let bad_ptr = 0xDEAD_BEEF as *const u32;
        let _boom = core::ptr::read_volatile(bad_ptr);
    }
}

extern "C" fn prog_magic(_: usize) {
    const MAGIC_VAL: u32 = 0xABAD_1DEA; // "A bad idea"
    static mut MAGIC_VAR: u32 = MAGIC_VAL;

    let addr = core::ptr::addr_of!(MAGIC_VAR) as usize;
    uprintln!("\n\rMagic variable is at 0x{:08X}", addr);

    // read_volatile forces compiler to read RAM
    while unsafe { core::ptr::read_volatile(addr as *const u32) } == MAGIC_VAL {
        whyos::sleep(10);
    }

    // reset it so the program can be run again
    unsafe { core::ptr::write_volatile(addr as *mut u32, MAGIC_VAL) };
    uprintln!("No more magic!");
}

extern "C" fn prog_top(mut delay: usize) {
    if delay == 0 { delay = 500; }

    // Hide cursor
    uprint!("\x1b[?25l");

    loop {
        // Move cursor to top-left
        uprint!("\x1b[H");

        uprintln!("\x1b[2K WhyOS top - Uptime: {} ticks (Refresh: {} ticks)", whyos::uptime_ticks(), delay);
        uprintln!("\x1b[2K  ID  | Name       | State     | Prio | Base       | SP         | Curr Use | Peak / Size (%)");
        uprintln!("\x1b[2K ─────+────────────+───────────+──────+────────────+────────────+──────────+──────────────────");

        let mut active_tasks = 0;

        for handle in whyos::allocated() {
            active_tasks += 1;

            if let Ok(info) = handle.info() {
                let name = info.name.unwrap_or("-");
                let stack_top = info.stack_base + info.stack_size;
                let current_usage = stack_top.saturating_sub(info.current_sp);
                let peak_pct = (info.max_stack_usage * 100) / info.stack_size;

                //  dynamic ANSI colors based on stack usage limits
                let color = if peak_pct >= 80 {
                    "\x1b[1;31m" // Bold Red
                } else if peak_pct >= 60 {
                    "\x1b[33m"  // Yellow
                } else {
                    "\x1b[32m"  // Green
                };
                let reset = "\x1b[0m";

                let prio_marker = if info.priority < 5 { "*" } else { " " }; // todo: idkkk

                uprintln!("\x1b[2K {:>4} | {:<10} | {:<9} | {}{:>3} | 0x{:08x} | 0x{:08x} | {:>6} B | {}{:>4} / {:<4} ({:>2}%){}",
                    info.handle.as_u32(),
                    name,
                    info.state,
                    prio_marker, info.priority,
                    info.stack_base,
                    info.current_sp,
                    current_usage,
                    color, info.max_stack_usage, info.stack_size, peak_pct, reset
                );
            }
        }

        uprintln!("\x1b[2K");
        uprintln!("\x1b[2K Total Active Tasks: {}", active_tasks);

        // Clear remaining lines from previous frame
        uprint!("\x1b[J");

        uprintln!("\x1b[2K \n(Press Ctrl+C to exit)");

        whyos::sleep(delay as u64);
    }
}


#[inline(never)] // Force real function calls to consume stack
fn pacman_eat(limit: usize) -> usize {
    let mut dummy_buffer = [0xAAu8; 128];
    let mut current_pct = 0;

    if let Ok(info) = whyos::my_handle().info() {
        let stack_top = info.stack_base + info.stack_size;
        let current_usage = stack_top.saturating_sub(info.current_sp);
        current_pct = (current_usage * 100) / info.stack_size;
    }

    if current_pct >= limit {
        dummy_buffer[0] = current_pct as u8;
        whyos::sleep(1000);
        return dummy_buffer[0] as usize;
    }

    whyos::sleep(500);
    let result = pacman_eat(limit);

    // Prevent compiler from optimizing the buffer away
    dummy_buffer[result % 128] as usize
}

extern "C" fn prog_eater(limit: usize) {
    uprintln!("Stack eater started...");
    pacman_eat(limit);
    uprintln!("Stack eater safely returned!");
}

pub static PROGRAMS: &[Program] = &[
    Program {
        name: "cnt",
        desc: "Counts down from N to 0",
        entry: prog_counter,
        default_arg: 10,
        priority: 2,
        stack_size: StackSize::SMALL
    },
    Program {
        name: "fib",
        desc: "Calculates N-th Fibonacci number (up to 186th)",
        entry: prog_fib,
        default_arg: 10,
        priority: 2,
        stack_size: StackSize::SMALL
    },
    Program {
        name: "tim",
        desc: "Sets a timer for N ticks",
        entry: prog_timer,
        default_arg: 10000,
        priority: 3,
        stack_size: StackSize::SMALL
    },
    Program {
        name: "panic",
        desc: "Triggers panic",
        entry: prog_panic,
        default_arg: 0,
        priority: 1,
        stack_size: StackSize::SMALL
    },
    Program {
        name: "hard",
        desc: "Triggers Hard Fault",
        entry: prog_hardfault,
        default_arg: 0,
        priority: 1,
        stack_size: StackSize::SMALL
    },
    Program {
        name: "magic",
        desc: "Wait for someone to poke memory to stop it",
        entry: prog_magic,
        default_arg: 0,
        priority: 3,
        stack_size: StackSize::SMALL
    },
    Program {
        name: "top",
        desc: "Live task view with N refresh ticks",
        entry: prog_top,
        default_arg: 500,
        priority: 2,
        stack_size: StackSize::DEFAULT
    },
    Program {
        name: "eater",
        desc: "Slowly eats its own stack up to ~N% - setting 95 or more will cause HardFault",
        entry: prog_eater,
        default_arg: 90,
        priority: 3,
        stack_size: StackSize::LARGE
    },
];