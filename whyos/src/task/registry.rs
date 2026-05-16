use crate::{TaskId, TaskState, WhyError, error::WhyResult, task::{TaskHandle, TaskStack}, utils::Bitmap};
use core::{num::NonZero, ops::{Index, IndexMut}};
use crate::scheduler::MAX_TASKS;

pub type TaskMask = u32;
pub type Gen = u8;

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

/// Wraps Bitmap
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct TaskMap(Bitmap<TaskMask>); // because MAX_TASKS=32, each bit is representing a task

impl TaskMap {
    pub const fn new() -> Self { Self(Bitmap::<TaskMask>::new()) }

    #[inline] pub fn add(&mut self, tid: TaskId) { self.0.set(tid.id()); }
    #[inline] pub fn remove(&mut self, tid: TaskId) { self.0.clear(tid.id()); }
    #[inline] pub fn is_set(&self, tid: TaskId) -> bool { self.0.is_set(tid.id()) }

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

pub struct Watchdog {
    remaining: u64,
    interval: NonZero<u64>
}

impl Watchdog {
    pub fn new(interval: NonZero<u64>) -> Self {
        Self { remaining: interval.get(), interval }
    }

    pub fn interval(&self) -> u64 { self.interval.get() }

    pub fn feed(&mut self) {
        self.remaining = self.interval.get()
    }

    pub fn check_n_tick(&mut self) -> bool {
        if self.remaining == 0 {
            true
        } else {
            self.remaining -= 1;
            false
        }
    }
}

// TODO: make these fields private?
pub struct Tcb { // task control block
    pub(crate) name: Option<&'static str>,
    pub(crate) state: TaskState,
    pub(crate) priority: u8, // lower number = higher priority
    pub(crate) stack: Option<TaskStack>,
    pub(crate) watchdog: Option<Watchdog>,
    pub(crate) generation: Gen
}

impl Tcb {
    pub const fn dead() -> Self {
        Self {
            name: None,
            state: TaskState::Dead,
            priority: u8::MAX,
            stack: None,
            watchdog: None,
            generation: Gen::MIN
        }
    }

    pub fn revive(&mut self, name: Option<&'static str>, priority: u8, stack: TaskStack) {
        *self = Self {
            name,
            state: TaskState::Ready,
            priority,
            stack: Some(stack),
            watchdog: None,
            generation: self.generation
        }
    }

    pub fn kill(&mut self) {
        *self = Self {
            name: None,
            state: TaskState::Dead,
            priority: u8::MAX,
            stack: None,
            watchdog: None,
            generation: self.generation.wrapping_add(1)
        }
    }

    //pub fn name(&self) -> Option<&'static str> { self.name }
    //pub fn state(&self) -> TaskState { self.state }
    //pub fn set_state(&mut self, state: TaskState) { self.state = state }
    //pub fn priority(&self) -> u8 { self.priority }
    //pub fn stack(&self) -> Option<&TaskStack> { self.stack.as_ref() }
    //pub fn watchdog(&self) -> Option<&Watchdog> { self.watchdog.as_ref() }
    //pub fn set_watchdog(&mut self, wd: Watchdog) { self.watchdog = Some(wd) }
    //pub fn generation(&self) -> Gen { self.generation }
}