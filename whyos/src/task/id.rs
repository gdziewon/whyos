use crate::{scheduler::MAX_TASKS};

#[derive(Clone, Copy, PartialEq, Eq, Debug, defmt::Format)]
#[repr(transparent)]
pub struct TaskId(usize); // todo: maybe should be u16, but usize is fast

impl TaskId {
    #[inline]
    pub const fn id(&self) -> usize {
        self.0
    }

    #[inline]
    pub fn new(id: usize) -> Option<Self> {
        if id >= MAX_TASKS {
            None
        } else {
            Some(Self(id))
        }
    }

    #[inline]
    pub(crate) const unsafe fn new_unchecked(id: usize) -> Self {
        Self(id)
    }
}