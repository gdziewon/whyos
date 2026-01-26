
#[repr(u32)]
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
}