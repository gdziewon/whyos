use core::{cell::{RefCell, UnsafeCell}, ops::{Deref, DerefMut}};
use critical_section::Mutex as CSMutex;

use crate::{itc::WaitQueue, scheduler};

pub struct Mutex<T> {
    data: UnsafeCell<T>,
    state: CSMutex<RefCell<MutexState>>
}

struct MutexState {
    locked: bool,
    waiting: WaitQueue
}

impl MutexState {
    const fn new() -> Self {
        Self {
            locked: false,
            waiting: WaitQueue::new()
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
            let acquired = critical_section::with(|cs| {
                let mut state = self.state.borrow_ref_mut(cs);

                if !state.locked { // success
                    state.locked = true;

                    state.waiting.remove_current(); // needed for weird stuff with suspend/resume FIXME
                    true

                } else { // we have to wait
                    state.waiting.block_current();
                    false
                }
            });

            if acquired {
                return MutexGuard { lock: self };
            }

            scheduler::yield_now();
        }
    }

    #[inline]
    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        critical_section::with(|cs| {
            let mut state = self.state.borrow_ref_mut(cs);

            if !state.locked {
                state.locked = true;
                Some(MutexGuard { lock: self })
            } else {
                None
            }
        })
    }

    #[inline]
    pub fn is_locked(&self) -> bool {
        critical_section::with(|cs| {
            self.state.borrow(cs).borrow().locked
        })
    }

    fn release(&self) {
        let someone_waiting = critical_section::with(|cs| {
            let mut state = self.state.borrow_ref_mut(cs);

            state.locked = false;

            state.waiting.wake_highest_prio()
        });

        // we might've woke someone with bigger priority, if not then scheduler will let us continue anyway
        if someone_waiting {
            scheduler::yield_now();
        }
    }
}