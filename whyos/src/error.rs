pub type WhyResult<T> = Result<T, WhyError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
#[repr(u8)]
pub enum WhyError {
    OutOfMemory,
    MaxTasksReached,
    InvalidOperation,
    InvalidTaskId,
    InternalError,
    InvalidHandle
}

impl core::fmt::Display for WhyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::OutOfMemory => write!(f, "Out of memory"),
            Self::MaxTasksReached => write!(f, "Maximum number of tasks reached"),
            Self::InvalidOperation => write!(f, "Invalid operation"),
            Self::InvalidTaskId => write!(f, "Invalid task ID"),
            Self::InternalError => write!(f, "Internal error"),
            Self::InvalidHandle => write!(f, "Invalid handle"),
        }
    }
}