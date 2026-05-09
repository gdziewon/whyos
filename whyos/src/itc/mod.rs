mod mutex;
mod queue;
mod sem;

pub use mutex::Mutex;
pub use queue::Queue;
pub use sem::Semaphore;

use crate::{TaskState, scheduler::Kernel, task::{TaskMap, BlockReason}};

// TODO: ADD SYSCALLS FOR ITC!!!!!

#[repr(transparent)]
struct WaitQueue {
    waiting: TaskMap
}

impl WaitQueue {
    pub const fn new() -> Self {
        Self { waiting: TaskMap::new() }
    }

    pub fn block_current(&mut self) {
        let curr = crate::current_tid();
        Kernel::lock(|k| {
            self.waiting.add(curr);
            k.block_task(curr, BlockReason::WaitQueue);
        })
    }

    pub fn remove_current(&mut self) {
        Kernel::lock(|k| {
            if let Some(curr) = k.current_task() {
                self.waiting.remove(curr);
            }
        })
    }

    // returns true if it woke up someone
    pub fn wake_highest_prio(&mut self) -> bool {
        Kernel::lock(|k| {

            let best_task = self.waiting
                .iter()
                .filter(|&tid|
                    matches!(
                        unsafe {k.task_unchecked(tid).state}, TaskState::Blocked(BlockReason::WaitQueue)
                    )
                ) // only wake tasks that are actually blocked by ITC (to avoid lost wakeup problem)
                .min_by_key(|&tid| unsafe { k.task_unchecked(tid).priority });

            if let Some(tid) = best_task {
                self.waiting.remove(tid);
                k.unblock_task(tid);
                true
            } else {
                false
            }
        })
    }
}