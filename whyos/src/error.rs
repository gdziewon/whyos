pub type WhyResult<T> = Result<T, WhyError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
#[repr(u8)]
pub enum WhyError {
    OutOfMemory = 1,
    MaxTasksReached = 2,
    InvalidOperation = 3,
    InvalidTaskId = 4,
    InternalError = 5,
    InvalidHandle = 6
}