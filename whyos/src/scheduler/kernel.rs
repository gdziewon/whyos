use core::num::NonZero;

use crate::{ResumeContext, TaskState, error::{WhyError, WhyResult}, scheduler::ContextSwitch, task::{BlockReason, TaskHandle, TaskId, TaskMap, TaskRegistry, TaskStack, Tcb, Watchdog}};

use super::idle::IdleTask;

pub type TaskMask = u32; // FIXME: right now, it can't go above u32, because of active_tasks syscall
pub const MAX_TASKS: usize = TaskMask::BITS as usize;

pub struct Kernel {
    registry: TaskRegistry,
    current_task: Option<TaskId>,
    system_ticks: u64,
    timer_interval: u32, // todo: should it be an option?
    idle: Option<IdleTask>,

    ready: TaskMap, // wants CPU
    blocked: TaskMap, // waiting for time
    zombies: TaskMap, // waiting to die // todo: we might not need this
}

impl Kernel {
    pub const fn new() -> Self {
        Self {
            registry: TaskRegistry::new(),
            current_task: None,
            system_ticks: 0,
            timer_interval: 0,
            idle: None,
            ready: TaskMap::new(),
            blocked: TaskMap::new(),
            zombies: TaskMap::new(),
        }
    }

    pub fn current_task(&self) -> Option<TaskId> { self.current_task }
    pub fn system_ticks(&self) -> u64 { self.system_ticks }
    pub fn set_timer_interval(&mut self, interval: u32) { self.timer_interval = interval }
    pub fn timer_interval(&self) -> u32 { self.timer_interval }
    pub fn idle_sp(&self) -> usize {
        self.idle.as_ref().expect("WhyOS: idle task not initialized").sp()
    }

    pub fn allocated(&self) -> TaskMap { self.registry.allocated_map() }
    pub fn handle(&self, tid: TaskId) -> WhyResult<TaskHandle> { self.registry.handle(tid) }
    pub fn task(&self, h: &TaskHandle) -> WhyResult<&Tcb> { self.registry.get_task(&h) }
    pub unsafe fn task_unchecked(&self, tid: TaskId) -> &Tcb { self.registry.get_task_unchecked(tid) }


    #[inline(always)]
    fn tick(&mut self) -> u64 {
        self.system_ticks += 1;
        self.system_ticks
    }

    pub fn on_tick(&mut self) -> u64 {
        let now = self.tick();

        // wake up sleeping tasks
        for tid in self.blocked.iter() {
            let task = self.registry.get_task_mut_unchecked(tid);

            if let TaskState::Blocked(BlockReason::Sleep(wakup_time)) = task.state {
                if wakup_time.get() <= now {
                    self.unblock_task(tid)
                }
            }
        }

        // software watchdog monitoring - ONLY FOR READY TASKS
        for tid in self.ready.iter() { // todo: watchdog should be behind feature
            self.wdt_check(tid);
        }

        now
    }

    fn wdt_check(&mut self, tid: TaskId) { // todo: maybe this should just return result?
        let task = self.registry.get_task_mut_unchecked(tid);

        if let Some(watchdog) = task.watchdog.as_mut() {
            if watchdog.check_n_tick() {
                defmt::warn!(
                    "WhyOS: Task {} didn't feed the watchdog for {} ticks - killing it",
                    tid.id(), watchdog.interval()
                );
                self.make_zombie(tid);
            }
        }
    }

    pub fn wdt_sub(&mut self, tid: TaskId, interval: NonZero<u64>) {
        let task = self.registry.get_task_mut_unchecked(tid);
        task.watchdog = Some(Watchdog::new(interval));
    }

    pub fn wdt_unsub(&mut self, tid: TaskId) {
        let task = self.registry.get_task_mut_unchecked(tid);
        task.watchdog = None;
    }

    pub fn wdt_feed(&mut self, tid: TaskId) {
        let task = self.registry.get_task_mut_unchecked(tid);
        if let Some(watchdog) = task.watchdog.as_mut() {
            watchdog.feed();
        }
    }

    pub fn block_task(&mut self, tid: TaskId, reason: BlockReason) -> ContextSwitch {
        let task = self.registry.get_task_mut_unchecked(tid);
        task.state = TaskState::Blocked(reason);
        self.blocked.add(tid);
        self.ready.remove(tid);

        ContextSwitch::yield_if(self.current_task == Some(tid)) // self-block
    }

    pub fn unblock_task(&mut self, tid: TaskId) {
        let task = self.registry.get_task_mut_unchecked(tid);

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


    /// COLD PATH with handles
    pub fn suspend_task(&mut self, h: &TaskHandle) -> WhyResult<ContextSwitch> {
        let task = self.registry.get_task_mut(&h)?;
        if task.state == TaskState::Zombie { // shouldn't suspend zombies
             return Err(WhyError::InvalidHandle);
        }

        // already suspended
        if let TaskState::Suspended(_) = task.state {
            return Ok(ContextSwitch::Continue);
        }

        let ctx: ResumeContext = task.state.try_into()?;
        task.state = TaskState::Suspended(ctx);

        self.ready.remove(h.tid());
        self.blocked.remove(h.tid());

        Ok(ContextSwitch::yield_if(self.current_task == Some(h.tid()))) // self-suspend
    }

    pub fn resume_task(&mut self, h: &TaskHandle) -> WhyResult<ContextSwitch> {
        let task = self.registry.get_task_mut(h)?;
        let tid = h.tid();

        if task.state == TaskState::Zombie { // shouldn't resume zombies
             return Err(WhyError::InvalidHandle);
        }

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
    ) -> WhyResult<TaskHandle> {

        let handle = self.registry.allocate(name, priority, stack)?;

        self.ready.add(handle.tid());

        Ok(handle)
    }

    pub fn kill_task(&mut self, h: &TaskHandle) -> WhyResult<ContextSwitch> {
        let tid = self.registry.validate(&h)?;
        Ok(self.make_zombie(tid))
    }

    pub fn init_idle(&mut self) -> usize {
        if self.idle.is_some() {
            panic!("WhyOS: idle already initialized");
        }

        self.idle = Some(IdleTask::new());
        self.idle_sp()
    }

    pub fn make_zombie(&mut self, tid: TaskId) -> ContextSwitch {
        self.ready.remove(tid);
        self.blocked.remove(tid);
        self.zombies.add(tid);

        let task = self.registry.get_task_mut_unchecked(tid);
        task.state = TaskState::Zombie;

        ContextSwitch::yield_if(self.current_task == Some(tid)) // self-remove
    }

    pub fn reap_zombies(&mut self) {
        for tid in self.zombies.iter() {
            self.zombies.remove(tid);
            let _ = self.registry.deallocate(tid);
        }
    }

    // returns SP of new taskget_task_mut_unchecked
    pub fn schedule(&mut self, old_sp: usize) -> usize {
        if let Some(curr) = self.current_task {
            let curr_task = self.registry.get_task_mut_unchecked(curr);
            if let Some(stack) = curr_task.stack.as_mut() {
                if !stack.check_canary() {
                    panic!("KERNEL PANIC: Stack Overflow detected in Task {}", curr.id());
                }

                stack.set_sp(old_sp);
            }

            // to not overwrite Blocked etc
            if curr_task.state == TaskState::Running {
                curr_task.state = TaskState::Ready;
            }
        } else {
            self.idle.as_mut().expect("WhyOS: idle task not initialized").set_sp(old_sp);
        }

        let next_task = self.pick_next();
        self.current_task = next_task;

        if let Some(next_task) = next_task {
            let next_task = self.registry.get_task_mut_unchecked(next_task);
            next_task.state = TaskState::Running; // we assume that all tasks in ready are Ready

            // safe, task in ready array mustn't be dead
            unsafe {
                next_task.stack.as_ref().unwrap_unchecked().sp()
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
            .min_by_key(|&tid| self.registry.get_task_unchecked(tid).priority)
    }
}