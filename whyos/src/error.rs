
pub type WhyResult<T> = Result<T, WhyError>;

pub trait ErrNo {
    fn to_errno(self) -> usize;
}

impl<T> ErrNo for WhyResult<T> {
    #[inline(always)]
    fn to_errno(self) -> usize {
        match self {
            Ok(_) => 0,
            Err(e) => e.id() as usize,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
#[repr(u8)]
pub enum WhyError {
    OutOfMemory = 1,
    MaxTasksReached = 2,
    InvalidOperation = 3,
    InvalidTaskId = 4,
    InternalError = u8::MAX
}

impl WhyError {
    #[inline(always)]
    pub const fn id(self) -> u8 {
        self as u8
    }
}

impl From<usize> for WhyError {
    fn from(id: usize) -> Self {
        match id {
            1 => WhyError::OutOfMemory,
            2 => WhyError::MaxTasksReached,
            3 => WhyError::InvalidOperation,
            4 => WhyError::InvalidTaskId,
            _ => WhyError::InternalError,
        }
    }
}

#[inline(always)]
pub fn from_errno(errno: usize) -> WhyResult<()> {
    if errno == 0 {
        Ok(())
    } else {
        Err(WhyError::from(errno))
    }
}