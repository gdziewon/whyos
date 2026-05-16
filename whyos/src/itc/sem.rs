use core::cell::RefCell;

use critical_section::Mutex as CSMutex;

use crate::{scheduler, itc::WaitQueue};
use crate::utils::log;

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
    pub const fn new(init_permits: usize, capacity: usize) -> Self {
        Self {
            state: CSMutex::new(RefCell::new(SemState::new(init_permits, capacity)))
        }
    }

    pub const fn binary() -> Self {
        Self::new(1, 1)
    }

    pub const fn counting(capacity: usize) -> Self {
        Self::new(capacity, capacity)
    }

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

    #[inline]
    pub fn available(&self) -> usize {
        critical_section::with(|cs| {
            self.state.borrow_ref(cs).permits
        })
    }
}