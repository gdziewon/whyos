

#[derive(Debug, Clone, Copy, PartialEq, Eq, defmt::Format)]
#[repr(u8)]
pub enum WhyError {
    OutOfMemory,
    MaxTasksReached,
    ResourceBusy, // todo: for try_lock
    InvalidOperation, // idk
}

pub type WhyResult<T> = core::result::Result<T, WhyError>;