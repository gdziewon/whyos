mod tcb;
mod stack;
mod map;
mod state;
mod builder;
mod info;

pub use tcb::Tcb;
pub use stack::{init_stack, TaskEntryPoint, STACK_CANARY};
pub use map::TaskMap;
pub use state::{TaskState, ResumeContext};
pub use builder::{TaskBuilder, TaskRoutine, StackSize};
pub use info::TaskInfo;

#[derive(Clone, Copy, PartialEq, Eq, Debug, defmt::Format)]
#[repr(transparent)]
pub struct TaskId(pub(crate) usize);

impl TaskId {
    #[inline]
    pub fn id(&self) -> usize {
        self.0
    }
}