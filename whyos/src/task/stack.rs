use core::{ptr, mem::{self, MaybeUninit}};

const EXC_RETURN_THREAD_PSP: u32 = 0xFFFFFFFD;
const XPSR_THUMB: u32 = 0x01000000;

pub const STACK_CANARY: u32 = 0xDEADC0DE; // todo: maybe something more random?

pub type TaskEntryPoint = extern "C" fn() -> !;

pub unsafe fn init_stack(
    stack_start: *mut u8,
    size: usize,
    entry_point: TaskEntryPoint
) -> usize {
    let stack_top = unsafe { stack_start.add(size) };

    let init_frame = InitStackFrame::new(entry_point);

    let frame_ptr =
        (stack_top as usize - mem::size_of::<InitStackFrame>())
        as *mut InitStackFrame;

    unsafe { ptr::write(frame_ptr, init_frame); }

    unsafe { *(stack_start as *mut u32) = STACK_CANARY; } // for stack overflow protection

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
            sw_frame: SwStackFrame::new(),
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
    fn new(entry_point: TaskEntryPoint) -> Self {
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
#[derive(Debug)]
struct SwStackFrame{ // R4-R11 + LR, popped manually in PendSV
    r4_11: MaybeUninit<[u32; 8]>,
    lr: u32,
}

impl SwStackFrame {
    fn new() -> Self {
        Self { r4_11: MaybeUninit::uninit(), lr: EXC_RETURN_THREAD_PSP }
    }
}
