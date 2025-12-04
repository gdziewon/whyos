use core::{cell::{RefCell, UnsafeCell}, ops::{Deref, DerefMut}};
use critical_section::Mutex as CSMutex;

use crate::{task::TaskList, scheduler, itc::pop_highest_prio};

pub struct Mutex<T> {
    data: UnsafeCell<T>,
    state: CSMutex<RefCell<MutexState>>
}

struct MutexState {
    locked: bool,
    owner: Option<usize>,
    waiting: TaskList
}

impl MutexState {
    const fn new() -> Self {
        Self {
            locked: false,
            owner: None,
            waiting: TaskList::new()
        }
    }
}

pub struct MutexGuard<'a, T> {
    lock: &'a Mutex<T>,
}

impl<T> Deref for MutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // its safe, as if we have MutexGuard, we own the lock
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> DerefMut for MutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // we own the lock
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Drop for MutexGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.release();
    }
}

// todo: check all the usages of "unsafe" and deduce if they are safe, leave comment explaining why
unsafe impl<T: Send> Sync for Mutex<T> {}

impl<T> Mutex<T> {
    pub const fn new(data: T) -> Self {
        Self {
            data: UnsafeCell::new(data),
            state: CSMutex::new(RefCell::new(MutexState::new()))
        }
    }

    // returns MutexGuard which releases the lock when it goes out of scope
    pub fn lock(&self) -> MutexGuard<'_, T> {
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
                    state.waiting.add(curr_tid);
                    scheduler::block_current_task();
                }
            });

            if resource_acquired {
                return MutexGuard { lock: self };
            } else {
                scheduler::yield_now()
            }
        }
    }

    fn release(&self) {
        let mut woken = false;

        critical_section::with(|cs| {
            let mut state = self.state.borrow(cs).borrow_mut();

            state.locked = false;
            state.owner = None;

            if let Some(tid) = pop_highest_prio(&mut state.waiting) {
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