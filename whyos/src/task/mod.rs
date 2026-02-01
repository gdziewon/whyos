mod builder;
mod info;
mod map;
pub mod ops;
mod stack;
mod state;
mod tcb;

pub use builder::{TaskBuilder, TaskRoutine, TaskRoutineArg, StackSize};
pub use info::TaskInfo;
pub use map::TaskMap;
pub use stack::{init_stack, TaskEntryPoint, STACK_CANARY};
pub use state::{TaskState, ResumeContext};
pub use tcb::Tcb;

use crate::{error::{WhyError, WhyResult}, scheduler::MAX_TASKS};

#[derive(Clone, Copy, PartialEq, Eq, Debug, defmt::Format)]
#[repr(transparent)]
pub struct TaskId(pub(crate) usize);

impl TaskId {
    #[inline]
    pub fn id(&self) -> usize {
        self.0
    }

    #[inline]
    pub fn new(id: usize) -> WhyResult<Self> {
        if id >= MAX_TASKS {
            Err(WhyError::InvalidTaskId)
        } else {
            Ok(Self(id))
        }
    }

    #[inline]
    pub const unsafe fn new_unchecked(id: usize) -> Self {
        Self(id)
    }
}