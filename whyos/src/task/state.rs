use core::fmt;

use crate::error::WhyError;


// inspired by https://freertos.org/Documentation/02-Kernel/02-Kernel-features/01-Tasks-and-co-routines/02-Task-states
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
#[repr(u8)]
pub enum TaskState {
    Ready,
    Running,
    Blocked,
    Sleeping,
    Suspended(ResumeContext),
    Zombie,
    Dead
}

impl fmt::Display for TaskState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use TaskState as Ts;
        match *self {
            Ts::Ready => f.pad("Ready"),
            Ts::Running => f.pad("Running"),
            Ts::Blocked => f.pad("Blocked"),
            Ts::Sleeping => f.pad("Sleeping"),
            Ts::Suspended(_) => f.pad("Suspended"),
            Ts::Zombie => f.pad("Zombie"),
            Ts::Dead => f.pad("Dead"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum ResumeContext {
    Ready,
    Blocked,
    Sleeping,
}

impl From<ResumeContext> for TaskState {
    #[inline]
    fn from(value: ResumeContext) -> Self {
        use TaskState as Ts;
        use ResumeContext as Rctx;
        match value {
            Rctx::Ready => Ts::Ready,
            Rctx::Blocked => Ts::Blocked,
            Rctx::Sleeping => Ts::Sleeping,
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
            Ts::Blocked => Ok(Rctx::Blocked),
            Ts::Sleeping => Ok(Rctx::Sleeping),
            _ => Err(WhyError::InvalidOperation)
        }
    }
}