mod builder;
mod id;
mod info;
mod map;
pub mod ops;
mod stack;
mod state;
mod table;
mod tcb;

pub use builder::{TaskBuilder, TaskRoutine, TaskRoutineArg, StackSize};
pub use id::TaskId;
pub use info::TaskInfo;
pub use map::TaskMap;
pub use stack::{Stack, TaskEntryPoint, TaskStack};
pub use state::{TaskState, BlockReason};
pub use table::TaskTable;
pub use tcb::{Tcb, Watchdog};

pub(crate) extern "C" fn task_exit_trampoline() -> ! {
	ops::kill_current_task();
	loop {
		cortex_m::asm::wfi();
	}
}