use num_enum::{TryFromPrimitive, IntoPrimitive};
use core::convert::TryFrom;

pub type WhyResult<T> = Result<T, WhyError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum WhyError {
    OutOfMemory = 1,
    MaxTasksReached = 2,
    InvalidOperation = 3,
    InvalidTaskId = 4,
    InternalError = 5,
}

impl From<usize> for WhyError {
    fn from(id: usize) -> Self {
        u8::try_from(id)
            .ok()
            .and_then(|code| WhyError::try_from(code).ok())
            .unwrap_or(WhyError::InternalError)
    }
}

impl From<WhyError> for usize {
    fn from(value: WhyError) -> Self {
        u8::from(value) as usize
    }
}