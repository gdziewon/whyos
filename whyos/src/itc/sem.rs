use core::cell::RefCell;

use critical_section::Mutex as CSMutex;

use crate::{scheduler, itc::WaitQueue};
use crate::utils::log;

/// Synchronization primitive typically used to signal events between tasks.
pub struct Semaphore {
    state: CSMutex<RefCell<SemState>>
}

struct SemState {
    permits: usize,
    capacity: usize,
    waiting: WaitQueue
}

unsafe impl Sync for Semaphore {}

impl SemState {
    const fn new(init_permits: usize, capacity: usize) -> Self {
        Self {
            permits: init_permits,
            capacity,
            waiting: WaitQueue::new()
        }
    }
}

impl Semaphore {
    /// Creates a semaphore with the given initial permit count and maximum capacity.
    ///
    /// When the semaphore is signalled, the permit count never exceeds `capacity`.
    pub const fn new(init_permits: usize, capacity: usize) -> Self {
        Self {
            state: CSMutex::new(RefCell::new(SemState::new(init_permits, capacity)))
        }
    }

    /// Creates a binary semaphore with one permit and capacity one.
    pub const fn binary() -> Self {
        Self::new(1, 1)
    }

    /// Creates a counting semaphore whose initial permit count equals its capacity.
    ///
    /// This is the same as calling `Semaphore::new(capacity, capacity)`.
    pub const fn counting(capacity: usize) -> Self {
        Self::new(capacity, capacity)
    }

    /// Consumes one permit, blocking the current task if there are no permits available.
    pub fn wait(&self) {
        loop {
            // yield if there are no permits
            let acquired = critical_section::with(|cs| {
                let mut state = self.state.borrow_ref_mut(cs);

                if state.permits > 0 { // some permits left
                    state.permits -= 1;
                    state.waiting.remove_current(); // needed for weird stuff with suspend/resume FIXME
                    log::debug!("Semaphore acquired, remaining permits {}", state.permits);
                    true
                } else { // NO PERMITS
                    state.waiting.block_current();
                    log::debug!("Semaphore blocking current task - no permits");
                    false
                }
            });

            if acquired {
                return;
            }

            scheduler::yield_now();
        }
    }

    /// Attempts to consume one permit without blocking.
    ///
    /// Returns `true` if a permit was acquired.
    #[inline]
    pub fn try_wait(&self) -> bool {
        critical_section::with(|cs| {
            let mut state = self.state.borrow_ref_mut(cs);

            if state.permits > 0 {
                state.permits -= 1;
                log::trace!("Semaphore try_wait success, remaining {}", state.permits);
                true
            } else {
                log::trace!("Semaphore try_wait failed, no permits");
                false
            }
        })
    }

    /// Adds one permit to the semaphore, waking the highest-priority waiter if needed.
    ///
    /// The permit count saturates at the semaphore's capacity.
    pub fn signal(&self) {
        // yield if we woke someone
        let someone_waiting = critical_section::with(|cs| {
            let mut state = self.state.borrow_ref_mut(cs);

            if state.permits < state.capacity {
                state.permits += 1;
                log::debug!("Semaphore signalled, permits {}", state.permits);
            }

            state.waiting.wake_highest_prio()
        });

        if someone_waiting {
            log::debug!("Semaphore signal woke waiting task");
            scheduler::yield_now();
        }
    }

    /// Returns the number of currently available permits.
    #[inline]
    pub fn available(&self) -> usize {
        critical_section::with(|cs| {
            self.state.borrow_ref(cs).permits
        })
    }
}