use core::cell::RefCell;

use critical_section::Mutex as CSMutex;

use crate::scheduler::{self, MAX_TASKS};

pub struct Semaphore {
    state: CSMutex<RefCell<SemState>>
}

struct SemState {
    permits: usize,
    capacity: usize,
    waiting: [bool; MAX_TASKS] // todo: let's focus on MAX_TASKS=32, change the array to u32 value
}

impl SemState {
    const fn new(init_permits: usize, capacity: usize) -> Self {
        Self {
            permits: init_permits,
            capacity,
            waiting: [false; MAX_TASKS]
        }
    }
}

impl Semaphore {
    pub const fn new(init_permits: usize, capacity: usize) -> Self {
        Self {
            state: CSMutex::new(RefCell::new(SemState::new(init_permits, capacity)))
        }
    }

    pub fn wait(&self) {
        loop {
            let mut permitted = false;
            critical_section::with(|cs| {
                let mut state = self.state.borrow(cs).borrow_mut();

                if state.permits > 0 { // some permits left
                    state.permits -= 1;
                    permitted = true;

                } else { // NO PERMITS
                    let curr_tid = scheduler::get_current_tid();
                    state.waiting[curr_tid] = true;
                    scheduler::block_current_task();
                }
            });

            if permitted {
                return;
            } else {
                scheduler::yield_now();
            }
        }
    }

    pub fn signal(&self) {
        let mut woken = false;
        critical_section::with(|cs| {
            let mut state = self.state.borrow(cs).borrow_mut();

            if state.permits < state.capacity {
                state.permits += 1;
            }

            let mut best_task: Option<usize> = None;
            let mut best_prio = u8::MAX;

            for tid in 1..MAX_TASKS {
                if state.waiting[tid] {
                    let prio = scheduler::get_task_priority(tid);

                    if prio < best_prio {
                        best_prio = prio;
                        best_task = Some(tid);
                    }
                }
            }

            if let Some(tid) = best_task {
                state.waiting[tid] = false;
                scheduler::wake_task(tid);
                woken = true;
            }
        });

        if woken {
            scheduler::yield_now();
        }
    }
}