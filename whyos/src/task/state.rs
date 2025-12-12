use crate::error::WhyError;


// inspired by https://freertos.org/Documentation/02-Kernel/02-Kernel-features/01-Tasks-and-co-routines/02-Task-states
#[derive(Clone, Copy, PartialEq, Eq, defmt::Format)]
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

#[derive(Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum ResumeContext {
    Ready,
    Blocked,
    Sleeping,
}

impl From<ResumeContext> for TaskState {
    #[inline]
    fn from(value: ResumeContext) -> Self {
        match value {
            ResumeContext::Ready => TaskState::Ready,
            ResumeContext::Blocked => TaskState::Blocked,
            ResumeContext::Sleeping => TaskState::Sleeping,
        }
    }
}

impl TryFrom<TaskState> for ResumeContext {
    type Error = WhyError;

    #[inline]
    fn try_from(value: TaskState) -> Result<Self, Self::Error> {
        use TaskState as TS;
        use ResumeContext as RCTX;
        match value {
            TS::Ready | TaskState::Running => Ok(RCTX::Ready),
            TS::Blocked => Ok(RCTX::Blocked),
            TS::Sleeping => Ok(RCTX::Sleeping),
            _ => Err(WhyError::InvalidOperation)
        }
    }
}