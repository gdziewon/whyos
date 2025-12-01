mod mutex;
mod queue;
mod sem;

pub use mutex::Mutex;
pub use queue::Queue;
pub use sem::Semaphore;

use crate::scheduler::{self, MAX_TASKS};

struct WaitList(u32); // because MAX_TASKS=32, each bit is representing a task

impl WaitList {
    const fn new() -> Self {
        Self(0)
    }

    #[inline]
    fn add(&mut self, tid: usize) {
        if tid < MAX_TASKS {
            self.0 |= 1 << tid;
        }
    }

    #[inline]
    fn remove(&mut self, tid: usize) {
        if tid < MAX_TASKS {
            self.0 &= !(1 << tid);
        }
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.0 == 0
    }

    fn pop_highest_prio(&mut self) -> Option<usize> {
        if self.is_empty() {
            return None;
        }

        let mut best_tid: Option<usize> = None;
        let mut best_prio = u8::MAX;

        // iterating over set bits
        let mut mask = self.0;

        while mask != 0 {
            let tid = mask.trailing_zeros() as usize; // find first set bit
            mask &= !(1 << tid); // clear it to find next one in next iteration

            let prio = scheduler::get_task_priority(tid);

            if prio < best_prio {
                best_prio = prio;
                best_tid = Some(tid);
            }
        }

        if let Some(tid) = best_tid {
            self.remove(tid); // pop
            Some(tid)
        } else {
            None
        }
    }
}