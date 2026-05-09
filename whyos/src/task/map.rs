use super::TaskId;
use crate::{scheduler::TaskMask, utils::Bitmap};

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct TaskMap(Bitmap<TaskMask>); // because MAX_TASKS=32, each bit is representing a task

impl TaskMap {
    pub const fn new() -> Self {
        Self(Bitmap::<TaskMask>::new())
    }

    #[inline]
    pub fn add(&mut self, tid: TaskId) {
        self.0.set(tid.id());
    }

    #[inline]
    pub fn remove(&mut self, tid: TaskId) {
        self.0.clear(tid.id());
    }

    #[inline]
    pub fn is_set(&self, tid: TaskId) -> bool {
        self.0.is_set(tid.id())
    }

    #[inline]
    pub fn first_free(&self) -> Option<TaskId> {
        self.0.first_unset().map(|tid| unsafe { TaskId::new_unchecked(tid) })
    }

    #[inline]
    pub fn iter(self) -> impl Iterator<Item = TaskId> {
        self.0.iter().map(|tid| unsafe { TaskId::new_unchecked(tid) })
    }

    #[inline]
    pub fn iter_from(self, start_bit: usize) -> impl Iterator<Item = TaskId> {
        self.0.iter_from(start_bit).map(|tid| unsafe { TaskId::new_unchecked(tid) })
    }
}