use core::{mem, ptr};

pub const EXC_RETURN_THREAD_PSP: u32 = 0xFFFFFFFD;
const XPSR_THUMB: u32 = 0x01000000;

pub type TaskEntryPoint = extern "C" fn() -> !;

// inspired by https://freertos.org/Documentation/02-Kernel/02-Kernel-features/01-Tasks-and-co-routines/02-Task-states
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Running,
    Blocked,
    Suspended, // todo: the suspend mechanism
    Zombie,
    Dead
}

#[derive(Clone, Copy)]
pub struct Tcb { // task control block
    pub sp: usize,
    pub state: TaskState,
    pub priority: u8, // lower number = higher priority
    pub wakeup_time: u64,
    pub stack_base: usize,
    pub stack_size: usize,
}

impl Tcb {
    pub const fn ready(sp: usize, priority: u8, stack_base: usize, stack_size: usize) -> Self {
        Self {
            sp,
            state: TaskState::Ready,
            priority,
            wakeup_time: 0,
            stack_base,
            stack_size
        }
    }

    pub const fn dead() -> Self {
        Self {
            sp: 0,
            state: TaskState::Dead,
            priority: u8::MAX,
            wakeup_time: 0,
            stack_base: 0,
            stack_size: 0
        }
    }
}

#[derive(Clone, Copy)]
pub struct TaskList(pub(crate) u32); // because MAX_TASKS=32, each bit is representing a task

impl TaskList {
    pub const fn new() -> Self {
        Self(0)
    }

    pub const fn from(value: u32) -> Self {
        Self(value)
    }

    #[inline]
    pub fn add(&mut self, tid: usize) {
        self.0 |= 1 << tid;
    }

    #[inline]
    pub fn remove(&mut self, tid: usize) {
        self.0 &= !(1 << tid);
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub fn iter(self) -> TaskListIter {
        TaskListIter { mask: self.0 }
    }
}

pub struct TaskListIter {
    mask: u32,
}

impl Iterator for TaskListIter {
    type Item = usize;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.mask == 0 {
            return None;
        }

        let tid = self.mask.trailing_zeros();

        self.mask &= !(1 << tid);

        Some(tid as usize)
    }
}

pub unsafe fn init_stack(
    stack_start: *mut u8,
    size: usize,
    entry_point: TaskEntryPoint
) -> usize {
    let stack_top = unsafe { stack_start.add(size) };

    let init_frame = InitStackFrame::new(entry_point);
    let frame_ptr = (stack_top as usize - mem::size_of::<InitStackFrame>()) as *mut InitStackFrame;
    unsafe { ptr::write(frame_ptr, init_frame) };

    frame_ptr as usize
}

#[repr(C)]
#[derive(Debug)]
struct InitStackFrame { // goes at the end of stack memory
    sw_frame: SwStackFrame,
    hw_frame: HwStackFrame
}

impl InitStackFrame {
    pub fn new(entry_point: TaskEntryPoint) -> Self {
        Self {
            sw_frame: SwStackFrame::default(),
            hw_frame: HwStackFrame::new(entry_point)
        }
    }
}

#[repr(C)]
#[derive(Debug)]
struct HwStackFrame { // popped automatically on interrupt return
    r0: u32,
    r1: u32,
    r2: u32,
    r3: u32,
    r12: u32,
    lr: u32,
    pc: u32,
    xpsr: u32,
}

impl HwStackFrame {
    pub fn new(entry_point: TaskEntryPoint) -> Self {
        Self {
            r0: 0, // todo: pass arg to task?
            r1: 0x11111111, // markers
            r2: 0x22222222,
            r3: 0x33333333,
            r12: 0xCCCCCCCC,
            lr: EXC_RETURN_THREAD_PSP, // on exception return, use psp in thread mode
            pc: entry_point as usize as u32,
            xpsr: XPSR_THUMB, // thumb bit, must be set for cortex-m
        }
    }
}

#[repr(C)]
#[derive(Debug, Default)]
struct SwStackFrame([u32; 8]); // R4-R11, popped manually in PendSV
