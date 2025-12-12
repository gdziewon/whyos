use super::{state::TaskState};

#[derive(defmt::Format)]
pub struct TaskInfo {
    pub id: usize,
    pub name: Option<&'static str>,
    pub state: TaskState,
    pub priority: u8,
    pub stack_size: usize,
}
