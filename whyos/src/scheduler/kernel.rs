use crate::task::{TaskId, TaskMap, TaskTable};


pub type TaskMask = u32; // FIXME: right now, it can't go above u32, because of active_tasks syscall
pub const MAX_TASKS: usize = TaskMask::BITS as usize;
pub const IDLE_TID: TaskId = unsafe { TaskId::new_unchecked(0) };

pub struct Kernel {
    pub tasks: TaskTable,
    pub current_task: TaskId,
    pub system_ticks: u64,

    pub allocated: TaskMap, // who exists
    pub ready: TaskMap, // wants CPU
    pub sleeping: TaskMap, // waiting for time
    pub zombies: TaskMap // waiting to die
}