use core::num::NonZero;

use crate::task::TaskStack;

use super::TaskState;

pub type Gen = u8;

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
    pub(crate) name: Option<&'static str>,
    pub(crate) state: TaskState,
    pub(crate) priority: u8, // lower number = higher priority
    pub(crate) stack: Option<TaskStack>,
    pub(crate) watchdog: Option<Watchdog>,
    pub(crate) generation: Gen
}

impl Tcb {
    pub const fn dead() -> Self {
        Self {
            name: None,
            state: TaskState::Dead,
            priority: u8::MAX,
            stack: None,
            watchdog: None,
            generation: Gen::MIN
        }
    }

    pub fn revive(&mut self, name: Option<&'static str>, priority: u8, stack: TaskStack) {
        *self = Self {
            name,
            state: TaskState::Ready,
            priority,
            stack: Some(stack),
            watchdog: None,
            generation: self.generation
        }
    }

    pub fn kill(&mut self) {
        *self = Self {
            name: None,
            state: TaskState::Dead,
            priority: u8::MAX,
            stack: None,
            watchdog: None,
            generation: self.generation.wrapping_add(1)
        }
    }

    //pub fn name(&self) -> Option<&'static str> { self.name }
    //pub fn state(&self) -> TaskState { self.state }
    //pub fn set_state(&mut self, state: TaskState) { self.state = state }
    //pub fn priority(&self) -> u8 { self.priority }
    //pub fn stack(&self) -> Option<&TaskStack> { self.stack.as_ref() }
    //pub fn watchdog(&self) -> Option<&Watchdog> { self.watchdog.as_ref() }
    //pub fn set_watchdog(&mut self, wd: Watchdog) { self.watchdog = Some(wd) }
    //pub fn generation(&self) -> Gen { self.generation }

}