use core::{cell::UnsafeCell, mem::MaybeUninit};

use critical_section::Mutex;
use crate::utils::Bitmap;

type PoolMask = u64;
const POOL_SIZE: usize = PoolMask::BITS as usize;
const BLOCK_SIZE: usize = 1024; // 1kb
const TOTAL_BYTES: usize = POOL_SIZE * BLOCK_SIZE; // 64kb

#[repr(C, align(8))]
struct MemoryPool {
    buffer: MaybeUninit<[u8; TOTAL_BYTES]>,
    bitmap: Bitmap<PoolMask>
}

unsafe impl Sync for MemoryPool {}

// simple bitmap allocator
static MEMORY: Mutex<UnsafeCell<MemoryPool>> = Mutex::new(UnsafeCell::new(MemoryPool {
    buffer: MaybeUninit::uninit(),
    bitmap: Bitmap::<u64>::new(),
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
    size: usize,
}

impl MemChunk for AllocatedMemory {
    #[inline]
    fn ptr(&self) -> *mut u8 { self.ptr }

    #[inline]
    fn size(&self) -> usize { self.size }
}

impl Drop for AllocatedMemory {
    fn drop(&mut self) {
        dealloc(self);
    }
}

unsafe impl Send for AllocatedMemory {}

// rounds up the size to multiple of 1024 (kb)
pub fn alloc(size: usize) -> Option<AllocatedMemory> { // todo: return a Result?
    let blocks = size.div_ceil(BLOCK_SIZE);

    if blocks == 0 || blocks > POOL_SIZE {
        return None; // todo: return result here?
    }

    critical_section::with(|cs| {
        let pool = unsafe {&mut *MEMORY.borrow(cs).get() };

        if let Some(start_idx) = pool.bitmap.find_first_fit(blocks) {
            pool.bitmap.set_range(start_idx, blocks); // found, mark as used

            let base_ptr = pool.buffer.as_mut_ptr() as *mut u8;
            let start_offset = start_idx * BLOCK_SIZE;
            let alloc_ptr = unsafe { base_ptr.add(start_offset) };

            let size = blocks * BLOCK_SIZE;
            return Some(AllocatedMemory { ptr: alloc_ptr, size});
        }
        None
    })
}

fn dealloc(chunk: &mut AllocatedMemory) {
    critical_section::with(|cs| {
        let pool = unsafe { &mut *MEMORY.borrow(cs).get() };
        let base_ptr = pool.buffer.as_mut_ptr() as *mut u8;

        let offset = chunk.ptr as usize - base_ptr as usize; // todo: wrapping sub?
        let start_bit = offset / BLOCK_SIZE;

        let blocks = chunk.size.div_ceil(BLOCK_SIZE);

        pool.bitmap.clear_range(start_bit, blocks);
    })
}