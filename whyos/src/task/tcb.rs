use super::TaskState;

#[derive(Clone, Copy)]
pub struct Tcb { // task control block
    pub name: Option<&'static str>,
    pub sp: usize,
    pub state: TaskState,
    pub priority: u8, // lower number = higher priority
    pub wakeup_time: u64,
    pub stack_base: usize,
    pub stack_size: usize
}

impl Tcb {
    pub const fn ready(name: Option<&'static str>, sp: usize, priority: u8, stack_base: usize, stack_size: usize) -> Self {
        Self {
            name,
            sp,
            state: TaskState::Ready,
            priority,
            wakeup_time: 0,
            stack_base,
            stack_size
        }
    }

    pub const fn dead() -> Self {
        Self {
            name: None,
            sp: 0,
            state: TaskState::Dead,
            priority: u8::MAX,
            wakeup_time: 0,
            stack_base: 0,
            stack_size: 0
        }
    }
}