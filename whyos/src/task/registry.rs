use crate::{TaskId, WhyError, error::WhyResult, task::{TaskMap, TaskStack, TaskTable, handle::TaskHandle}};
use super::Tcb;


pub struct TaskRegistry {
    tasks: TaskTable,
    allocated: TaskMap
}

impl TaskRegistry {
    pub const fn new() -> Self {
        Self { tasks: TaskTable::new(), allocated: TaskMap::new() }
    }

    pub fn allocate(
        &mut self,
        name: Option<&'static str>,
        priority: u8,
        stack: TaskStack
    ) -> WhyResult<TaskHandle> {
        let tid = self.allocated
            .first_free()
            .ok_or(WhyError::MaxTasksReached)?;

        self.allocated.add(tid);
        self.tasks[tid].revive(name, priority, stack);

        Ok(TaskHandle::new(tid, self.tasks[tid].generation))
    }

    pub fn deallocate(&mut self, tid: TaskId) -> WhyResult<()> {
        self.allocated.remove(tid);
        self.tasks[tid].kill();
        Ok(())
    }

    #[inline]
    pub fn validate(&self, h: &TaskHandle) -> WhyResult<TaskId> {
        let tid = h.tid();

        if !self.allocated.is_set(tid) {
            return Err(WhyError::InvalidHandle);
        }

        if self.tasks[tid].generation != h.generation() {
            return Err(WhyError::InvalidHandle);
        }

        Ok(tid)
    }

    pub fn handle(&self, tid: TaskId) -> WhyResult<TaskHandle> {
        if !self.allocated.is_set(tid) {
            return Err(WhyError::InvalidTaskId);
        }

        Ok(TaskHandle::new(tid, self.tasks[tid].generation))
    }

    pub fn allocated_map(&self) -> TaskMap { self.allocated }

    pub fn get_task(&self, h: &TaskHandle) -> WhyResult<&Tcb> {
        let tid = self.validate(h)?;
        Ok(&self.tasks[tid])
    }

    pub fn get_task_mut(&mut self, h: &TaskHandle) -> WhyResult<&mut Tcb> {
        let tid = self.validate(h)?;
        Ok(&mut self.tasks[tid])
    }


    // hot path for kernel
    #[inline]
    pub fn get_task_unchecked(&self, tid: TaskId) -> &Tcb {
        &self.tasks[tid]
    }

    #[inline]
    pub fn get_task_mut_unchecked(&mut self, tid: TaskId) -> &mut Tcb {
        &mut self.tasks[tid]
    }
}