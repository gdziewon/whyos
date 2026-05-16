use crate::task::Tcb;
use crate::{scheduler::MAX_TASKS};
use crate::{error::WhyResult, scheduler::{self, ContextSwitch, Kernel}, task::registry::Gen};


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

    /// Packs handle into a single u32
    /// Layout: [ 16 bits reserved ] [ 8 bits generation ] [ 8 bits tid ]
    #[inline]
    pub fn as_u32(&self) -> u32 {
        let tid_val = self.tid.id() as u32; // fixme: this assumes gen and tid are both 1byte
        let gen_val = self.generation as u32;

        (gen_val << 8) | tid_val
    }

    /// Attempts to reconstruct a TaskHandle from a packed u32
    #[inline]
    pub fn from_u32(raw: u32) -> Option<Self> {
        let tid_val = (raw & 0xFF) as usize;
        let gen_val = ((raw >> 8) & 0xFF) as Gen;

        TaskId::new(tid_val).map(|tid| Self {
            tid,
            generation: gen_val,
        })
    }

    pub fn suspend(&self) -> WhyResult<()> {
        if Kernel::lock(|k| {
            k.suspend_task(self)
        })? == ContextSwitch::Yield {
            scheduler::yield_now();
        }

        Ok(())
    }

    pub fn resume(&self) -> WhyResult<()> {
        if Kernel::lock(|k| {
            k.resume_task(self)
        })? == ContextSwitch::Yield {
            scheduler::yield_now();
        }

        Ok(())
    }

    /// Immediately kills the specified task and reclaims its memory.
    ///
    /// **WARNING:** This function does NOT run destructors. If the task
    /// holds a locked `Mutex` or other shared resources, they will remain
    /// locked forever.
    pub fn kill(&self) -> WhyResult<()> {
        if Kernel::lock(|k| {
            k.kill_task(self)
        })? == ContextSwitch::Yield {
            scheduler::yield_now();
        }

        Ok(())
    }

    pub fn info(self) -> WhyResult<TaskInfo> {
        Kernel::lock(|k| {
            let task = k.task(&self)?;
            TaskInfo::new(self, task)
        })
    }
}

#[repr(C)]
pub struct TaskInfo {
    pub handle: TaskHandle,
    pub name: Option<&'static str>,
    pub state: TaskState,
    pub priority: u8,
    pub current_sp: usize,
    pub stack_base: usize,
    pub stack_size: usize,
    pub max_stack_usage: usize
}

impl TaskInfo {
    pub fn new(handle: TaskHandle, task: &Tcb) -> WhyResult<Self> {
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

use core::{fmt, num::NonZero};

use crate::error::WhyError;


// inspired by https://freertos.org/Documentation/02-Kernel/02-Kernel-features/01-Tasks-and-co-routines/02-Task-states
#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum BlockReason {
    Sleep(NonZero<u64>),
    WaitQueue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum TaskState {
    Ready,
    Running,
    Blocked(BlockReason),
    Suspended(ResumeContext),
    Zombie,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
pub enum ResumeContext {
    Ready,
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