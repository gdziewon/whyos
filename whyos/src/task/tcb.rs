use core::num::NonZero;

use crate::task::TaskStack;

use super::TaskState;

pub struct Watchdog {
    remaining: u64,
    interval: NonZero<u64>
}

impl Watchdog {
    pub fn new(interval: NonZero<u64>) -> Self {
        Self { remaining: interval.get(), interval }
    }

    pub fn feed(&mut self) {
        self.remaining = self.interval.get()
    }

    pub fn check_n_tick(&mut self) -> bool {
        if self.remaining == 0 {
            true
        } else {
            self.remaining -= 1;
            false
        }
    }

    pub fn interval(&self) -> u64 { self.interval.get() }
}

// TODO: make these fields private?
pub struct Tcb { // task control block
    pub name: Option<&'static str>,
    pub state: TaskState,
    pub priority: u8, // lower number = higher priority
    pub stack: Option<TaskStack>,
    pub watchdog: Option<Watchdog>
}

impl Tcb {
    pub const fn ready(name: Option<&'static str>, priority: u8, stack: TaskStack) -> Self {
        Self {
            name,
            state: TaskState::Ready,
            priority,
            stack: Some(stack),
            watchdog: None
        }
    }

    pub const fn dead() -> Self {
        Self {
            name: None,
            state: TaskState::Dead,
            priority: u8::MAX,
            stack: None,
            watchdog: None
        }
    }
}