use core::{cell::UnsafeCell, mem::MaybeUninit};

use critical_section::Mutex;
use crate::utils::{MultiBitmap, log};
use crate::arch::{KernelArch as _, TargetArch};

const BLOCK_SIZE: usize = 1024; // 1kb
const MAX_BLOCKS: usize = TargetArch::HEAP_KB;

const BITMAP_WORDS: usize = MAX_BLOCKS.div_ceil(64); // how many u64 words we need
const TOTAL_BYTES: usize = MAX_BLOCKS * BLOCK_SIZE;

#[repr(C, align(8))]
struct MemoryPool {
    buffer: MaybeUninit<[u8; TOTAL_BYTES]>,
    bitmap: MultiBitmap<BITMAP_WORDS>,
}


unsafe impl Sync for MemoryPool {}

// simple bitmap allocator
static MEMORY: Mutex<UnsafeCell<MemoryPool>> = Mutex::new(UnsafeCell::new(MemoryPool {
    buffer: MaybeUninit::uninit(),
    bitmap: MultiBitmap::new(),
}));


pub trait MemChunk {
    fn ptr(&self) -> *mut u8;
    fn size(&self) -> usize;
}

pub(crate) struct StaticMemory {
    ptr: *mut u8,
    size: usize,
}

impl StaticMemory {
    #[inline]
    pub const unsafe fn from_raw(ptr: *mut u8, size: usize) -> Self {
        Self { ptr, size }
    }
}

impl MemChunk for StaticMemory {
    #[inline]
    fn ptr(&self) -> *mut u8 { self.ptr }

    #[inline]
    fn size(&self) -> usize { self.size }
}

unsafe impl Send for StaticMemory {}

pub struct AllocatedMemory {
    ptr: *mut u8,
    size: usize, // always a multiple of BLOCK_SIZE
}

impl MemChunk for AllocatedMemory {
    #[inline] fn ptr(&self) -> *mut u8 { self.ptr }
    #[inline] fn size(&self) -> usize { self.size }
}

impl Drop for AllocatedMemory {
    fn drop(&mut self) {
        dealloc(self);
    }
}

unsafe impl Send for AllocatedMemory {}

// rounds up the size to multiple of 1024 (kb)
pub fn alloc(size: usize) -> Option<AllocatedMemory> {
    let blocks = size.div_ceil(BLOCK_SIZE);
    if blocks == 0 || blocks > MAX_BLOCKS {
        log::warn!("Memory alloc request invalid: {} bytes", size);
        return None;
    }

    critical_section::with(|cs| {
        let pool = unsafe { &mut *MEMORY.borrow(cs).get() };

        pool.bitmap.find_first_fit(blocks).map(|start| {
            pool.bitmap.set_range(start, blocks);
            let ptr = unsafe {
                pool.buffer.as_mut_ptr().cast::<u8>().add(start * BLOCK_SIZE)
            };
            log::debug!("Memory alloc: {} bytes ({} blocks) at start {}", size, blocks, start);
            AllocatedMemory { ptr, size: blocks * BLOCK_SIZE }
        })
    })
}

fn dealloc(chunk: &mut AllocatedMemory) {
    critical_section::with(|cs| {
        let pool = unsafe { &mut *MEMORY.borrow(cs).get() };
        let base = pool.buffer.as_mut_ptr() as usize;
        let start_bit = (chunk.ptr as usize - base) / BLOCK_SIZE;
        let blocks = chunk.size / BLOCK_SIZE; // always exact
        pool.bitmap.clear_range(start_bit, blocks);
        log::debug!("Memory dealloc: {} bytes ({} blocks) at start {}", chunk.size, blocks, start_bit);
    });
}