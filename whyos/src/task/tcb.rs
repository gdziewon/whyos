use crate::task::Stack;

use super::TaskState;

pub struct Tcb { // task control block
    pub name: Option<&'static str>,
    pub state: TaskState,
    pub priority: u8, // lower number = higher priority
    pub wakeup_time: u64,
    pub stack: Option<Stack>,
    pub watchdog_remaining_ticks: Option<u64>,
    pub watchdog_interval_ticks: u64
}

impl Tcb {
    pub const fn ready(name: Option<&'static str>, priority: u8, stack: Stack) -> Self {
        Self {
            name,
            state: TaskState::Ready,
            priority,
            wakeup_time: 0,
            stack: Some(stack),
            watchdog_remaining_ticks: None,
            watchdog_interval_ticks: 0
        }
    }

    pub const fn dead() -> Self {
        Self {
            name: None,
            state: TaskState::Dead,
            priority: u8::MAX,
            wakeup_time: 0,
            stack: None,
            watchdog_remaining_ticks: None,
            watchdog_interval_ticks: 0
        }
    }
}