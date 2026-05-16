use core::{cell::{RefCell, UnsafeCell}, ops::{Deref, DerefMut}};
use critical_section::Mutex as CSMutex;

use crate::{itc::WaitQueue, scheduler};
use crate::utils::log;

/// A mutual exclusion primitive useful for protecting shared data.
///
/// This mutex will block tasks waiting for the lock to become available.
/// The highest priority task will be woken up first when the lock is released.
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

/// An RAII implementation of a "scoped lock" of a mutex. When this structure is
/// dropped (falls out of scope), the lock will be unlocked.
///
/// The data protected by the mutex can be accessed through this guard via its
/// `Deref` and `DerefMut` implementations.
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
    /// Creates a new `Mutex` in an unlocked state containing the given data.
    pub const fn new(data: T) -> Self {
        Self {
            data: UnsafeCell::new(data),
            state: CSMutex::new(RefCell::new(MutexState::new()))
        }
    }

    /// Acquires a mutex, blocking the current task until it is able to do so.
    ///
    /// This function will block the calling task until it is available to acquire the mutex.
    /// Upon returning, the task is the only one with the mutex held.
    pub fn lock(&self) -> MutexGuard<'_, T> {
        // loop is needed since if the acquisition fails, task should check from the start
        loop {
            let acquired = critical_section::with(|cs| {
                let mut state = self.state.borrow_ref_mut(cs);

                if !state.locked { // success
                    state.locked = true;

                    state.waiting.remove_current(); // needed for weird stuff with suspend/resume FIXME
                    log::debug!("Mutex acquired");
                    true

                } else { // we have to wait
                    state.waiting.block_current();
                    log::debug!("Mutex lock failed, blocking current task");
                    false
                }
            });

            if acquired {
                return MutexGuard { lock: self };
            }

            scheduler::yield_now();
        }
    }

    /// Attempts to acquire this `lock` without blocking.
    #[inline]
    pub fn try_lock(&self) -> Option<MutexGuard<'_, T>> {
        critical_section::with(|cs| {
            let mut state = self.state.borrow_ref_mut(cs);

            if !state.locked {
                state.locked = true;
                log::trace!("Mutex try_lock success");
                Some(MutexGuard { lock: self })
            } else {
                log::trace!("Mutex try_lock failed - locked");
                None
            }
        })
    }

    /// Checks if the lock is currently held by any task.
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
            log::debug!("Mutex released - woke waiting task");
            scheduler::yield_now();
        } else {
            log::trace!("Mutex released - no waiting tasks");
        }
    }
}