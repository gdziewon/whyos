use super::state::TaskState;

#[derive(defmt::Format)]
#[repr(C)]
pub struct TaskInfo {
    pub id: usize,
    pub name: Option<&'static str>,
    pub state: TaskState,
    pub priority: u8,
    pub current_sp: usize,
    pub stack_base: usize,
    pub stack_size: usize,
    pub max_stack_usage: usize
}
