use crate::{error::{WhyError, WhyResult}, scheduler::MAX_TASKS};

#[derive(Clone, Copy, PartialEq, Eq, Debug, defmt::Format)]
#[repr(transparent)]
pub struct TaskId(usize);

impl TaskId {
    #[inline]
    pub const fn id(&self) -> usize {
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