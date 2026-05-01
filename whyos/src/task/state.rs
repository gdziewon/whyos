use core::{fmt, num::NonZero};

use crate::error::WhyError;


// inspired by https://freertos.org/Documentation/02-Kernel/02-Kernel-features/01-Tasks-and-co-routines/02-Task-states
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum BlockReason {
    Sleep(NonZero<u64>),
    WaitQueue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum TaskState {
    Ready,
    Running,
    Blocked(BlockReason),
    Suspended(ResumeContext),
    Zombie,
    Dead
}

impl fmt::Display for TaskState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use TaskState as Ts;
        let s = match *self {
            Ts::Ready => "Ready",
            Ts::Running => "Running",
            Ts::Blocked(_) => "Blocked",
            Ts::Suspended(_) => "Suspended",
            Ts::Zombie => "Zombie",
            Ts::Dead => "Dead",
        };
        f.write_str(s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum ResumeContext {
    Ready,
    Blocked(BlockReason),
}

impl From<ResumeContext> for TaskState {
    #[inline]
    fn from(value: ResumeContext) -> Self {
        use TaskState as Ts;
        use ResumeContext as Rctx;
        match value {
            Rctx::Ready => Ts::Ready,
            Rctx::Blocked(reason) => Ts::Blocked(reason),
        }
    }
}

impl TryFrom<TaskState> for ResumeContext {
    type Error = WhyError;

    #[inline]
    fn try_from(value: TaskState) -> Result<Self, Self::Error> {
        use TaskState as Ts;
        use ResumeContext as Rctx;
        match value {
            Ts::Ready | TaskState::Running => Ok(Rctx::Ready),
            Ts::Blocked(reason) => Ok(Rctx::Blocked(reason)),
            _ => Err(WhyError::InvalidOperation)
        }
    }
}