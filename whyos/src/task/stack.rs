use core::{mem::{self, MaybeUninit}, ptr};

use crate::memory::MemChunk;

const EXC_RETURN_THREAD_PSP: u32 = 0xFFFFFFFD;
const XPSR_THUMB: u32 = 0x01000000;

pub const STACK_CANARY: u32 = 0xDEADC0DE; // todo: maybe something more random?
pub const STACK_PAINT: u32 = 0xFEEDFACE;

pub type TaskEntryPoint = extern "C" fn(usize);

// TODO: IMPLEMENT Stack struct

pub unsafe fn init_stack(
    stack: &MemChunk,
    entry_point: TaskEntryPoint,
    arg: usize,
    return_handler: usize
) -> usize {
    // painting for usage calculation
    let stack_u32 = stack.ptr() as *mut u32;
    let paint_count = stack.size() / size_of::<u32>();
    for i in 0..paint_count {
        unsafe { ptr::write_volatile(stack_u32.add(i), STACK_PAINT) };
    }

    let stack_top = unsafe { stack.ptr().add(stack.size()) };

    let init_frame = InitStackFrame::new(entry_point as usize, arg, return_handler);

    let frame_ptr =
        (stack_top as usize - mem::size_of::<InitStackFrame>())
        as *mut InitStackFrame;

    unsafe { ptr::write(frame_ptr, init_frame); }

    unsafe { *stack_u32 = STACK_CANARY; } // for stack overflow protection

    frame_ptr as usize
}

pub fn calculate_stack_usage(mem: &MemChunk) -> usize {
    let ptr = mem.ptr() as *const u32;
    let count = mem.size() / core::mem::size_of::<u32>();

    // start at 1 to skip canary
    for i in 1..count {
        let val = unsafe { ptr::read_volatile(ptr.add(i)) };

        // found a word that's not paint, we hit the used portion of a stack
        if val != STACK_PAINT {
            let unused_words = i;
            let used_bytes = mem.size() - (unused_words * 4);
            return used_bytes;
        }
    }

    // stack completly full or corrupted
    mem.size()
}

#[repr(C)]
#[derive(Debug)]
struct InitStackFrame { // goes at the end of stack memory
    sw_frame: SwStackFrame,
    hw_frame: HwStackFrame
}

impl InitStackFrame {
    pub fn new(entry_point: usize, arg: usize, return_handler: usize) -> Self {
        Self {
            sw_frame: SwStackFrame::new(),
            hw_frame: HwStackFrame::new(entry_point, arg, return_handler)
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
    fn new(entry_point: usize, arg: usize, return_handler: usize) -> Self {
        Self {
            r0: arg as u32,
            r1: 0x11111111, // markers
            r2: 0x22222222,
            r3: 0x33333333,
            r12: 0xCCCCCCCC,
            lr: return_handler as u32, // on exception return, use psp in thread mode
            pc: entry_point as u32,
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
