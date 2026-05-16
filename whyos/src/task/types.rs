use core::fmt;
use core::num::NonZero;

use crate::WhyError;
use crate::task::Tcb;
use crate::{scheduler::MAX_TASKS};
use crate::{error::WhyResult, scheduler::{self, ContextSwitch, Kernel}, task::registry::Gen};


#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[repr(transparent)]
pub struct TaskId(usize); // maybe should be u16, but usize is fast

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

/// A safe, unique reference to a spawned task within the OS.
///
/// The handle contains both a Task ID (`tid`) and a `generation` counter.
/// This combination guarantees memory safety and prevents the ABA problem.
#[derive(Debug)]
pub struct TaskHandle {
    tid: TaskId,
    generation: Gen
}

impl core::fmt::Display for TaskHandle {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.as_u32())
    }
}

impl TaskHandle {
    pub(crate) fn new(tid: TaskId, generation: Gen) -> Self {
        Self { tid, generation }
    }

    pub(crate) fn tid(&self) -> TaskId {
        self.tid
    }

    pub(crate) fn generation(&self) -> Gen {
        self.generation
    }

    /// Packs the handle into a single `u32` value.
    ///
    /// # Layout
    /// `[ 16 bits reserved ] [ 8 bits generation ] [ 8 bits tid ]`
    #[inline]
    pub fn as_u32(&self) -> u32 {
        let tid_val = self.tid.id() as u32; // fixme: this assumes gen and tid are both 1byte
        let gen_val = self.generation as u32;

        (gen_val << 8) | tid_val
    }

    /// Attempts to reconstruct a `TaskHandle` from a packed `u32` value.
    ///
    /// **Note:** This method does not verify if the task is currently alive in the scheduler.
    #[inline]
    pub fn from_u32(raw: u32) -> Option<Self> {
        let tid_val = (raw & 0xFF) as usize;
        let gen_val = ((raw >> 8) & 0xFF) as Gen;

        TaskId::new(tid_val).map(|tid| Self {
            tid,
            generation: gen_val,
        })
    }

    /// Pauses the execution of the referenced task.
    ///
    /// A suspended task will not be scheduled for execution until [`TaskHandle::resume`]
    /// is explicitly called.
    /// If a task suspends itself, a context switch will occur immediately.
    pub fn suspend(&self) -> WhyResult<()> {
        if Kernel::lock(|k| {
            k.suspend_task(self)
        })? == ContextSwitch::Yield {
            scheduler::yield_now();
        }

        Ok(())
    }

    /// Resumes a previously suspended task.
    ///
    /// The task is placed back into the scheduling queue. If its priority is higher
    /// than the currently running task, a context switch will occur immediately.
    pub fn resume(&self) -> WhyResult<()> {
        if Kernel::lock(|k| {
            k.resume_task(self)
        })? == ContextSwitch::Yield {
            scheduler::yield_now();
        }

        Ok(())
    }

    /// Kills the specified task, marking it for cleanup.
    ///
    /// Fails if task doesn't exists or has already been killed.
    /// If the killed task is the currently running task, a context switch
    /// occurs immediately.
    /// It's memory will be later reclaimed by idle task.
    ///
    /// # Deadlocks
    /// **WARNING:** This function terminates the task instantly and does **NOT** run
    /// destructors (`Drop` implementations). If the task holds locked `Mutex`,
    /// or other shared resources, they will remain locked forever, which may lead to
    /// system-wide deadlocks.
    pub fn kill(&self) -> WhyResult<()> {
        if Kernel::lock(|k| {
            k.kill_task(self)
        })? == ContextSwitch::Yield {
            scheduler::yield_now();
        }

        Ok(())
    }

    /// Retrieves a snapshot of the task's runtime information.
    pub fn info(self) -> WhyResult<TaskInfo> {
        Kernel::lock(|k| {
            let task = k.task(&self)?;
            TaskInfo::new(self, task)
        })
    }
}

/// A snapshot of a task's internal state and statistics.
#[repr(C)]
pub struct TaskInfo {
    /// The handle belonging to the inspected task.
    pub handle: TaskHandle,
    /// The optional, human-readable name of the task.
    pub name: Option<&'static str>,
    /// The current scheduling state of the task
    pub state: TaskState,
    /// The task's priority level
    pub priority: u8,
    /// The current Stack Pointer (SP) address.
    pub current_sp: usize,
    /// The base (lowest) address of the allocated stack memory.
    pub stack_base: usize,
    /// The total size of the allocated stack in bytes.
    pub stack_size: usize,
    /// The maximum number of bytes the stack has used so far.
    pub max_stack_usage: usize
}

impl TaskInfo {
    pub(crate) fn new(handle: TaskHandle, task: &Tcb) -> WhyResult<Self> {
        if let Some(stack) = &task.stack {
            Ok(TaskInfo {
                handle,
                name: task.name,
                state: task.state,
                priority: task.priority,
                current_sp: stack.sp(),
                stack_base: stack.base() as usize,
                stack_size: stack.size(),
                max_stack_usage: stack.usage(),
            })
        } else {
            Err(WhyError::InvalidTaskId)
        }
    }
}


/// Specifies the condition preventing a blocked task from being scheduled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum BlockReason {
    /// The task is sleeping until specified `system_ticks` amount.
    Sleep(NonZero<u64>),
    /// The task is blocked waiting for an Inter-Task Communication (ITC)
    WaitQueue,
}

/// Represents the current scheduling state of a task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum TaskState {
    /// The task is ready to run.
    Ready,
    /// The task is currently executing.
    Running,
    /// The task is waiting for an event.
    Blocked(BlockReason),
    /// The task was explicitly paused via an API call.
    Suspended(ResumeContext),
    /// The task has finished execution or was killed, but its memory resources
    /// have not yet been fully reclaimed.
    Zombie,
    /// The task slot is completely empty and free to be reallocated to a new task.
    Dead
}

impl fmt::Display for TaskState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use TaskState as Ts;
        let s = match *self {
            Ts::Ready => "Ready",
            Ts::Running => "Running",
            Ts::Blocked(_) => "Blocked",
            Ts::Suspended(_) => "Suspended",
            Ts::Zombie => "Zombie",
            Ts::Dead => "Dead",
        };
        f.write_str(s)
    }
}

/// Stores the original state of a task before it was explicitly suspended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum ResumeContext {
    /// The task was ready to execute before being suspended.
    Ready,
    /// The task was blocked on a specific condition before being suspended.
    Blocked(BlockReason),
}

impl From<ResumeContext> for TaskState {
    #[inline]
    fn from(value: ResumeContext) -> Self {
        use TaskState as Ts;
        use ResumeContext as Rctx;
        match value {
            Rctx::Ready => Ts::Ready,
            Rctx::Blocked(reason) => Ts::Blocked(reason),
        }
    }
}

impl TryFrom<TaskState> for ResumeContext {
    type Error = WhyError;

    #[inline]
    fn try_from(value: TaskState) -> Result<Self, Self::Error> {
        use TaskState as Ts;
        use ResumeContext as Rctx;
        match value {
            Ts::Ready | TaskState::Running => Ok(Rctx::Ready),
            Ts::Blocked(reason) => Ok(Rctx::Blocked(reason)),
            _ => Err(WhyError::InvalidOperation)
        }
    }
}