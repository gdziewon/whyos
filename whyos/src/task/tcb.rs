use crate::task::TaskStack;

use super::TaskState;

pub struct Watchdog {
    pub remaining: u64,
    pub interval: u64
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