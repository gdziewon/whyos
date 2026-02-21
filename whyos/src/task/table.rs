use core::ops::{Index, IndexMut};
use crate::scheduler::MAX_TASKS;
use super::{Tcb, TaskId};

pub struct TaskTable([Tcb; MAX_TASKS]);

impl TaskTable {
    pub const fn new() -> Self {
        Self([const { Tcb::dead() }; MAX_TASKS])
    }
}

impl Index<TaskId> for TaskTable {
    type Output = Tcb;

    #[inline(always)]
    fn index(&self, index: TaskId) -> &Self::Output {
        // SAFETY: TaskId is guaranteed to be < (MAX_TASKS) by its constructor
        unsafe { self.0.get_unchecked(index.id()) }
    }
}

impl IndexMut<TaskId> for TaskTable {
    #[inline(always)]
    fn index_mut(&mut self, index: TaskId) -> &mut Self::Output {
        unsafe { self.0.get_unchecked_mut(index.id()) }
    }
}