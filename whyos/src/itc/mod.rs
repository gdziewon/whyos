mod mutex;
mod queue;
mod sem;

pub use mutex::Mutex;
pub use queue::Queue;
pub use sem::Semaphore;

use crate::{scheduler, task::TaskMap};

fn pop_highest_prio(list: &mut TaskMap) -> Option<usize> {
    if list.is_empty() {
        return None;
    }

    let mut best_task = None;
    let mut best_prio = u8::MAX;

    for tid in list.iter() {
        let prio = scheduler::get_task_priority(tid);
        if prio < best_prio {
            best_prio = prio;
            best_task = Some(tid);
        }
    }

    if let Some(tid) = best_task {
        list.remove(tid);
        best_task
    } else {
        None
    }
}