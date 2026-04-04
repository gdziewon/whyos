mod mutex;
mod queue;
mod sem;

pub use mutex::Mutex;
pub use queue::Queue;
pub use sem::Semaphore;

use crate::{TaskState, scheduler::Kernel, task::TaskMap};

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
        Kernel::lock(|k| {
            let curr = k.current_task().expect("WhyOS: idle cannot block on wait queues");
            self.waiting.add(curr);
            k.block_task(curr);
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
                .filter(|&tid| k.task(tid).state == TaskState::Blocked) // only wake tasks that are actually blocked (to avoid lost wakeup problem)
                .min_by_key(|&tid| k.task(tid).priority);

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