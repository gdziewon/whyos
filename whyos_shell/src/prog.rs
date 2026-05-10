use whyos::{StackSize, TaskRoutineArg};

use crate::uprintln;

pub struct Program {
    pub name: &'static str,
    pub desc: &'static str,
    pub entry: TaskRoutineArg<usize>,
    pub default_arg: usize,
    pub priority: u8,
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

    uprintln!("{}", a);
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

pub static PROGRAMS: &[Program] = &[ // todo: add more programs
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
    }
];