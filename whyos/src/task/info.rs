use crate::{WhyError, error::WhyResult, task::{TaskHandle, Tcb}};

use super::state::TaskState;

#[repr(C)]
pub struct TaskInfo {
    pub handle: TaskHandle,
    pub name: Option<&'static str>,
    pub state: TaskState,
    pub priority: u8,
    pub current_sp: usize,
    pub stack_base: usize,
    pub stack_size: usize,
    pub max_stack_usage: usize
}

impl TaskInfo {
    pub fn new(handle: TaskHandle, task: &Tcb) -> WhyResult<Self> {
        if let Some(stack) = &task.stack {
            Ok(TaskInfo {
                handle,
                name: task.name,
                state: task.state,
                priority: task.priority,
                current_sp: stack.sp(),
                stack_base: stack.base() as usize,
                stack_size: stack.size(),
                max_stack_usage: stack.usage(),
            })
        } else {
            Err(WhyError::InvalidTaskId)
        }
    }
}