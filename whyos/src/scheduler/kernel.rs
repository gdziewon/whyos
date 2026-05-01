use core::num::NonZero;

use crate::{ResumeContext, TaskState, error::{WhyError, WhyResult}, scheduler::ContextSwitch, task::{BlockReason, TaskId, TaskMap, TaskStack, TaskTable, Tcb, Watchdog}};

use super::idle::IdleTask;

pub type TaskMask = u32; // FIXME: right now, it can't go above u32, because of active_tasks syscall
pub const MAX_TASKS: usize = TaskMask::BITS as usize;

pub struct Kernel {
    tasks: TaskTable,
    current_task: Option<TaskId>,
    system_ticks: u64,
    idle: Option<IdleTask>,

    allocated: TaskMap, // who exists
    ready: TaskMap, // wants CPU
    blocked: TaskMap, // waiting for time
    zombies: TaskMap, // waiting to die
}

impl Kernel {
    pub const fn new() -> Self {
        Self {
            tasks: TaskTable::new(),
            current_task: None,
            system_ticks: 0,
            idle: None,
            allocated: TaskMap::new(),
            ready: TaskMap::new(),
            blocked: TaskMap::new(),
            zombies: TaskMap::new(),
        }
    }

    pub fn task(&self, tid: TaskId) -> &Tcb { &self.tasks[tid] }
    pub fn current_task(&self) -> Option<TaskId> { self.current_task }
    pub fn system_ticks(&self) -> u64 { self.system_ticks }
    pub fn idle_sp(&self) -> usize {
        self.idle.as_ref().expect("WhyOS: idle task not initialized").sp()
    }

    pub fn allocated(&self) -> TaskMap { self.allocated }
    pub fn ready(&self) -> TaskMap { self.ready }
    pub fn blocked(&self) -> TaskMap { self.blocked }
    pub fn zombies(&self) -> TaskMap { self.zombies }

    // TODO: add one, central state change function, add checks on each state change, possibly TaskRegistry struct with bitmaps
    //fn set_task_state(&mut self, tid: TaskId, new_state: TaskState) {}

    pub fn tick(&mut self) -> u64 {
        self.system_ticks += 1;
        self.system_ticks
    }

    pub fn wdt_check(&mut self, tid: TaskId) { // todo: maybe this should just return result?
        let task = &mut self.tasks[tid];

        if let Some(watchdog) = task.watchdog.as_mut() {
            if watchdog.check_n_tick() {
                defmt::warn!(
                    "WhyOS: Task {} didn't feed the watchdog for {} ticks - killing it",
                    tid.id(), watchdog.interval()
                );
                self.make_zombie(tid).unwrap();
            }
        }
    }

    pub fn wdt_sub(&mut self, tid: TaskId, interval: NonZero<u64>) {
        let task = &mut self.tasks[tid];

        task.watchdog = Some(Watchdog::new(interval));
    }

    pub fn wdt_unsub(&mut self, tid: TaskId) {
        let task = &mut self.tasks[tid];

        task.watchdog = None;
    }

    pub fn wdt_feed(&mut self, tid: TaskId) {
        let task = &mut self.tasks[tid];

        if let Some(watchdog) = task.watchdog.as_mut() {
            watchdog.feed();
        }
    }

    pub fn block_task(&mut self, tid: TaskId, reason: BlockReason) -> ContextSwitch {
        self.tasks[tid].state = TaskState::Blocked(reason);
        self.blocked.add(tid);
        self.ready.remove(tid);

        ContextSwitch::yield_if(self.current_task == Some(tid)) // self-block
    }

    pub fn unblock_task(&mut self, tid: TaskId) {
        let task = &mut self.tasks[tid];

        if let TaskState::Blocked { .. } = task.state {
            task.state = TaskState::Ready;
            self.blocked.remove(tid);
            self.ready.add(tid);
        } else {
            panic!("WhyOS: Waking non blocked task, should never happen")
        }
    }

    pub fn sleep_task(&mut self, tid: TaskId, ticks: NonZero<u64>) -> ContextSwitch {
        let target_time = self.system_ticks.saturating_add(ticks.get()); // in case of overflow, eternal sleep

        // systime >=0, ticks >= 1 and saturating add, so this is safe
        let wakeup_time = unsafe { NonZero::new_unchecked(target_time) };

        self.block_task(tid, BlockReason::Sleep(wakeup_time)) // self-sleep
    }

    pub fn suspend_task(&mut self, tid: TaskId) -> WhyResult<ContextSwitch> {
        if !self.allocated.is_set(tid) {
            return Err(WhyError::InvalidTaskId);
        }

        let task = &mut self.tasks[tid];

        // already suspended
        if let TaskState::Suspended(_) = task.state {
            return Ok(ContextSwitch::Continue);
        }

        let ctx: ResumeContext = task.state.try_into()?;
        task.state = TaskState::Suspended(ctx);

        self.ready.remove(tid);
        self.blocked.remove(tid);

        Ok(ContextSwitch::yield_if(self.current_task == Some(tid))) // self-suspend
    }

    pub fn resume_task(&mut self, tid: TaskId) -> WhyResult<ContextSwitch> {
        if !self.allocated.is_set(tid) {
            return Err(WhyError::InvalidTaskId);
        }

        let task = &mut self.tasks[tid];

        let TaskState::Suspended(ctx) = task.state else {
            return Ok(ContextSwitch::Continue);
        };

        match ctx {
            ResumeContext::Ready => {
                task.state = TaskState::Ready;
                self.ready.add(tid);
                Ok(ContextSwitch::Yield)
            },
            ResumeContext::Blocked(reason @ BlockReason::Sleep(wakeup_time)) => {
                if wakeup_time.get() <= self.system_ticks { // time to wake up
                    task.state = TaskState::Ready;
                    self.ready.add(tid);
                    Ok(ContextSwitch::Yield)
                } else {
                    task.state = TaskState::Blocked(reason);
                    self.blocked.add(tid);
                    Ok(ContextSwitch::Continue)
                }
            },
            ResumeContext::Blocked(_reason @ BlockReason::WaitQueue) => {
                task.state = TaskState::Ready; // if blocked on waitqueue, go to ready - issues with lost wakeup
                self.ready.add(tid);
                Ok(ContextSwitch::Yield)
            }
        }
    }

    pub fn spawn_task(
        &mut self,
        name: Option<&'static str>,
        priority: u8,
        stack: TaskStack
    ) -> WhyResult<TaskId> {

        let tid = self.allocated
            .first_free()
            .ok_or(WhyError::MaxTasksReached)?;

        self.init_task(tid, name, priority, stack);

        Ok(tid)
    }

    pub fn init_task(
        &mut self,
        tid: TaskId,
        name: Option<&'static str>,
        priority: u8,
        stack: TaskStack
    ) {
        self.allocated.add(tid); // TODO: create some set_state() method that handles all of these changes
        self.ready.add(tid);

        self.tasks[tid] = Tcb::ready(name, priority, stack);
    }

    pub fn init_idle(&mut self) {
        if self.idle.is_some() {
            panic!("WhyOS: idle already initialized");
        }

        self.idle = Some(IdleTask::new());
    }

    pub fn make_zombie(&mut self, tid: TaskId) -> WhyResult<ContextSwitch> {
        if !self.allocated.is_set(tid) {
            return Err(WhyError::InvalidTaskId);
        }
        self.ready.remove(tid);
        self.blocked.remove(tid);

        self.zombies.add(tid);
        self.tasks[tid].state = TaskState::Zombie;

        Ok(ContextSwitch::yield_if(self.current_task == Some(tid))) // self-remove
    }

    pub fn remove_zombie(&mut self, tid: TaskId) -> Option<TaskStack> {
        if let Some(stack) = self.tasks[tid].stack.take() {
            self.zombies.remove(tid);
            self.allocated.remove(tid);

            self.tasks[tid] = Tcb::dead();
            Some(stack)
        } else {
            None
        }
    }

    // returns SP of new task
    pub fn schedule(&mut self, old_sp: usize) -> usize {
        if let Some(curr) = self.current_task {
            if let Some(stack) = self.tasks[curr].stack.as_mut() {
                if !stack.check_canary() {
                    panic!("KERNEL PANIC: Stack Overflow detected in Task {}", curr.id());
                }

                stack.set_sp(old_sp);
            }

            // to not overwrite Blocked etc
            if self.tasks[curr].state == TaskState::Running {
                self.tasks[curr].state = TaskState::Ready;
            }
        } else {
            self.idle.as_mut().expect("WhyOS: idle task not initialized").set_sp(old_sp);
        }

        let next_task = self.pick_next();
        self.current_task = next_task;

        if let Some(next_task) = next_task {
            self.tasks[next_task].state = TaskState::Running; // we assume that all tasks in ready are Ready

            // TODO: MAKE ABSOLUTE SURE IF THIS IS SAFE
            // Should be safe, task in ready array mustn't be dead
            unsafe {
                self.tasks[next_task].stack.as_ref().unwrap_unchecked().sp()
            }
        } else {
            self.idle.as_ref().expect("WhyOS: idle task not initialized").sp()
        }
    }

    fn pick_next(&self) -> Option<TaskId> {
        // start searching from (current + 1) for round robin
        let next = match self.current_task {
            Some(curr) => (curr.id() + 1) % MAX_TASKS,
            None => 0,
        };

        self.ready
            .iter_from(next)
            .min_by_key(|&tid| self.tasks[tid].priority)
    }
}