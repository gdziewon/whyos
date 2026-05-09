mod builder; // todo: to much mods
mod id;
mod info;
mod map;
pub mod ops;
mod stack;
mod state;
mod table;
mod tcb;
mod registry;
mod handle;

pub use builder::{TaskBuilder, TaskRoutine, TaskRoutineArg, StackSize};
pub use id::TaskId;
pub use info::TaskInfo;
pub use map::TaskMap;
pub use stack::{Stack, TaskEntryPoint, TaskStack};
pub use state::{TaskState, ResumeContext, BlockReason};
pub use table::TaskTable;
pub use tcb::{Tcb, Watchdog};
pub use registry::TaskRegistry;
pub use handle::TaskHandle;

pub(crate) extern "C" fn task_exit_trampoline() -> ! {
	crate::exit()
}