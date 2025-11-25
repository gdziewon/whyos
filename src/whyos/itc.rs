use core::cell::{RefCell, UnsafeCell};
use critical_section::Mutex as CSMutex;

use crate::whyos::scheduler::{self, MAX_TASKS};


pub struct Mutex<T> {
    data: UnsafeCell<T>,
    state: CSMutex<RefCell<MutexState>>
}

unsafe impl<T: Send> Sync for Mutex<T> {}

struct MutexState {
    locked: bool,
    owner: Option<usize>,
    waiting: [bool; MAX_TASKS] // task sets it's index to true if it's waiting
}

impl MutexState {
    const fn new() -> Self {
        Self {
            locked: false,
            owner: None,
            waiting: [false; MAX_TASKS]
        }
    }
}

impl<T> Mutex<T> {
    pub const fn new(data: T) -> Self {
        Self {
            data: UnsafeCell::new(data),
            state: CSMutex::new(RefCell::new(MutexState::new()))
        }
    }

    pub fn lock(&self) -> &mut T { // todo: implement MutexGuard
        // loop is needed since if the acquisition fails, task should check from the start
        loop {
            // variable to track what to do after interrupts are enabled
            let mut resource_acquired = false;

            critical_section::with(|cs| {
                let mut state = self.state.borrow(cs).borrow_mut();
                let curr_tid = scheduler::get_current_tid();

                if !state.locked { // success
                    state.locked = true;
                    state.owner = Some(curr_tid);
                    resource_acquired = true;
                } else { // we have to wait
                    state.waiting[curr_tid] = true;
                    scheduler::block_current_task();
                }
            });

            if resource_acquired {
                return unsafe { &mut *self.data.get()} // we got mutex, return
            } else {
                scheduler::yield_now()
            }
        }
    }

    pub fn unlock(&self) {
        let mut woken = false;

        critical_section::with(|cs| {
            let mut state = self.state.borrow(cs).borrow_mut();

            state.locked = false;
            state.owner = None;

            let mut best_task: Option<usize> = None;
            let mut best_prio = u8::MAX;

            for tid in 1..MAX_TASKS { // todo: maybe linked-list instead of map?
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

        // we might've woke someone with bigger priority, if not then scheduler will let us continue anyway
        if woken {
            scheduler::yield_now();
        }
    }
}