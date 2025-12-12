
pub type WhyResult<T> = Result<T, WhyError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
#[repr(u8)]
pub enum WhyError {
    OutOfMemory,
    MaxTasksReached,
    InvalidOperation,
    InvalidTaskId
}