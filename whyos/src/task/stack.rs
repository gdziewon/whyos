use core::ptr;

use crate::memory::{AllocatedMemory, MemChunk};
use crate::arch::InitStackFrame;

pub const STACK_CANARY: u32 = 0xDEADC0DE; // todo: maybe something more random?
pub const STACK_PAINT: u32 = 0xFEEDFACE;

pub type TaskEntryPoint = extern "C" fn(usize);


pub struct Stack<M: MemChunk> {
    mem: M,
    sp: usize,
}

unsafe impl<M: MemChunk + Send> Send for Stack<M> {}

pub type TaskStack = Stack<AllocatedMemory>;

impl<M: MemChunk> Stack<M> {
    pub fn init(mem: M, entry: TaskEntryPoint, arg: usize, ret: usize) -> Self {
        assert_eq!(mem.ptr() as usize % 8, 0, "WhyOS: stack memory must be 8-byte aligned");

        // painting for usage calculation
        let stack_u32 = mem.ptr() as *mut u32;
        let paint_count = mem.size() / core::mem::size_of::<u32>();
        for i in 0..paint_count {
            unsafe { ptr::write_volatile(stack_u32.add(i), STACK_PAINT) };
        }

        let stack_top = unsafe { mem.ptr().add(mem.size()) };
        let init_frame = InitStackFrame::new(entry as usize, arg, ret);
        let frame_ptr = (stack_top as usize - core::mem::size_of::<InitStackFrame>()) as *mut InitStackFrame;

        unsafe { ptr::write(frame_ptr, init_frame); }
        unsafe { *stack_u32 = STACK_CANARY; }

        Self { mem, sp: frame_ptr as usize }
    }

    #[inline]
    pub fn sp(&self) -> usize {
        self.sp
    }

    #[inline]
    pub fn set_sp(&mut self, sp: usize) {
        self.sp = sp
    }

    #[inline]
    pub fn size(&self) -> usize {
        self.mem.size()
    }

    #[inline]
    pub fn base(&self) -> *const u8 {
        self.mem.ptr()
    }

    /// Returns amount of used words
    pub fn usage(&self) -> usize {
        let ptr = self.mem.ptr() as *const u32;
        let count = self.mem.size() / core::mem::size_of::<u32>();

        // start at 1 to skip canary
        for i in 1..count {
            let val = unsafe { ptr::read_volatile(ptr.add(i)) };

            // found a word that's not paint, we hit the used portion of a stack
            if val != STACK_PAINT {
                let unused_words = i;
                let used_bytes = self.mem.size() - (unused_words * 4);
                return used_bytes;
            }
        }

        // stack completly full or corrupted
        self.mem.size()
    }

    #[inline]
    pub fn check_canary(&self) -> bool {
        unsafe {
            core::ptr::read_volatile(self.mem.ptr() as *const u32) == STACK_CANARY
        }
    }
}