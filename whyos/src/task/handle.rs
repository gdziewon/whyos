use crate::{TaskId, TaskInfo, error::WhyResult, scheduler::{self, ContextSwitch, Kernel}, task::tcb::Gen};

#[derive(Debug)] // todo: do we need it?
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
        let tid_val = self.tid.id() as u32; // fixme: this assumes gen and tid fit in 8bits
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