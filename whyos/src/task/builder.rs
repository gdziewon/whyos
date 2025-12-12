use super::{TaskEntryPoint, TaskId};
use crate::error::WhyResult;
use crate::scheduler;

pub struct TaskBuilder {
    entry: TaskEntryPoint,
    priority: u8,
    stack_size: usize,
    name: Option<&'static str>
}

impl TaskBuilder {
    #[inline]
    pub fn new(entry: TaskEntryPoint) -> Self {
        Self {
            entry,
            priority: 128,
            stack_size: 1024, // 1Kb // todo: add StackSize struct
            name: None,
        }
    }

    #[inline]
    pub fn priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    #[inline]
    pub fn stack_size(mut self, size: usize) -> Self {
        self.stack_size = size;
        self
    }

    #[inline]
    pub fn name(mut self, name: &'static str) -> Self {
        self.name = Some(name);
        self
    }

    #[inline]
    pub fn spawn(self) -> WhyResult<TaskId> {
        scheduler::add_task(self.entry, self.name, self.priority, self.stack_size)
    }
}