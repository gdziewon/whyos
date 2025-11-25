use core::{cell::UnsafeCell, mem::{self, MaybeUninit}, ptr};


pub const EXC_RETURN_THREAD_PSP: u32 = 0xFFFFFFFD;
const XPSR_THUMB: u32 = 0x01000000;

pub type TaskEntryPoint = extern "C" fn() -> !;

// inspired by https://freertos.org/Documentation/02-Kernel/02-Kernel-features/01-Tasks-and-co-routines/02-Task-states
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Running,
    Blocked,
    Suspended
}

#[derive(Clone, Copy)]
pub struct Tcb { // task control block
    pub sp: u32,
    pub state: TaskState,
    pub priority: u8, // lower number = higher priority
    pub wakeup_time: u64
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

#[repr(C, align(8))]
pub struct Stack<const SIZE: usize> {
    data: UnsafeCell<MaybeUninit<[u8; SIZE]>>, //
}

unsafe impl<const S: usize> Sync for Stack<S> {}

impl<const SIZE: usize> Stack<SIZE> {
    pub const fn new() -> Self {
        Self {
            data: UnsafeCell::new(MaybeUninit::uninit()),
        }
    }

    // fixme: api should be cleaner
    pub fn init(&self, entry_point: TaskEntryPoint) -> u32 {
        let stack_ptr = self.data.get() as *mut u8;
        let stack_top = unsafe { stack_ptr.add(SIZE) };

        let init_frame = InitStackFrame::new(entry_point);
        let frame_ptr = (stack_top as usize - mem::size_of::<InitStackFrame>()) as *mut InitStackFrame;

        unsafe { ptr::write(frame_ptr, init_frame) };
        frame_ptr as u32
    }
}