use core::cell::RefCell;

use critical_section::Mutex as CSMutex;

use crate::{task::TaskList, scheduler, itc::pop_highest_prio};

pub struct Semaphore {
    state: CSMutex<RefCell<SemState>>
}

struct SemState {
    permits: usize,
    capacity: usize,
    waiting: TaskList
}

unsafe impl Sync for Semaphore {}

impl SemState {
    const fn new(init_permits: usize, capacity: usize) -> Self {
        Self {
            permits: init_permits,
            capacity,
            waiting: TaskList::new()
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
                    state.waiting.add(curr_tid);
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

            if let Some(tid) = pop_highest_prio(&mut state.waiting) {
                scheduler::wake_task(tid);
                woken = true;
            }
        });

        if woken {
            scheduler::yield_now();
        }
    }
}