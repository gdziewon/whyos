use core::{fmt, num::NonZero};


// inspired by https://freertos.org/Documentation/02-Kernel/02-Kernel-features/01-Tasks-and-co-routines/02-Task-states
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum BlockReason {
    Sleep(NonZero<u64>),
    WaitQueue(u32),
    Suspended
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum TaskState {
    Ready,
    Running,
    Blocked(BlockReason),
    Zombie,
    Dead
}

impl fmt::Display for TaskState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use TaskState as Ts;
        match *self {
            Ts::Ready => f.pad("Ready"),
            Ts::Running => f.pad("Running"),
            Ts::Blocked { .. } => f.pad("Blocked"),
            Ts::Zombie => f.pad("Zombie"),
            Ts::Dead => f.pad("Dead"),
        }
    }
}