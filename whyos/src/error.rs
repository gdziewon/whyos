use num_enum::{TryFromPrimitive, IntoPrimitive};
use core::convert::TryFrom;

pub type WhyResult<T> = Result<T, WhyError>;

pub(crate) const SUCCESS: usize = 0;


pub trait ErrNo {
    fn to_errno(self) -> usize;
}

impl<T> ErrNo for WhyResult<T> {
    #[inline(always)]
    fn to_errno(self) -> usize {
        match self {
            Ok(_) => 0,
            Err(e) => e.into(),
        }
    }
}

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

#[inline(always)]
pub fn from_errno(errno: usize) -> WhyResult<()> {
    if errno == SUCCESS {
        Ok(())
    } else {
        Err(WhyError::from(errno))
    }
}