#[cfg(feature = "defmt")]
pub(crate) use defmt::{debug, error, info, trace, warn};

#[cfg(not(feature = "defmt"))]
macro_rules! trace {
    ($($arg:tt)*) => {};
}

#[cfg(not(feature = "defmt"))]
macro_rules! debug {
    ($($arg:tt)*) => {};
}

#[cfg(not(feature = "defmt"))]
macro_rules! info {
    ($($arg:tt)*) => {};
}

#[cfg(not(feature = "defmt"))]
macro_rules! _warn {
    ($($arg:tt)*) => {};
}

#[cfg(not(feature = "defmt"))]
macro_rules! error {
    ($($arg:tt)*) => {};
}

#[cfg(not(feature = "defmt"))]
pub(crate) use trace;

#[cfg(not(feature = "defmt"))]
pub(crate) use debug;

#[cfg(not(feature = "defmt"))]
pub(crate) use info;

#[cfg(not(feature = "defmt"))]
pub(crate) use _warn as warn; // because Rust has this #[warn()]

#[cfg(not(feature = "defmt"))]
pub(crate) use error;
