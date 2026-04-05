mod svc;
mod calls;

use num_enum::{TryFromPrimitive, IntoPrimitive};

pub use calls::*;

#[repr(C)]
pub struct SpawnArgs {
    pub entry: usize,
    pub arg: usize,
    pub name_ptr: *const u8,
    pub name_len: usize,
    pub stack_size: usize,
    pub priority: u8,
}

#[derive(TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum SvcNumber {
    Start = 0,
    Yield = 1,
    Sleep = 2,
    Exit = 3,
    Suspend = 4,
    Resume = 5,
    GetTaskInfo = 6,
    GetCurrentTid = 7,
    GetCurrentName = 8,
    GetUptimeTicks = 9,
    GetTaskCount = 10,
    GetActiveTasks = 11,
    ReclaimMemory = 13,
    WatchdogSubscribe = 14,
    WatchdogUnsubscribe = 15,
    WatchdogFeed = 16,
    Spawn = 17,
    Kill = 18,
    Reboot = 19,
}

impl SvcNumber {
    #[inline(always)]
    pub const fn id(self) -> u8 {
        self as u8
    }
}