#[cfg(target_arch = "arm")]
mod cortex_m;
#[cfg(target_arch = "arm")]
pub use cortex_m::*;

#[cfg(target_arch = "riscv32")]
mod riscv;
#[cfg(target_arch = "riscv32")]
pub use riscv::*;