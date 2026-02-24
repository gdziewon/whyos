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

pub struct MemChunk {
    ptr: *mut u8,
    size: usize,
}

impl MemChunk {
    #[inline]
    pub fn ptr(&self) -> *mut u8 { self.ptr }

    #[inline]
    pub fn size(&self) -> usize { self.size }
}

unsafe impl Send for MemChunk {}

// rounds up the size to multiple of 1024 (kb)
pub fn alloc(size: usize) -> Option<MemChunk> { // todo: return a Result?
    let blocks = size.div_ceil(BLOCK_SIZE);

    if blocks == 0 || blocks > POOL_SIZE {
        return None; // todo: return result here?
    }

    // let search_mask: u64 = if blocks == 64 {
    //     u64::MAX // to handle edge case of 64 - shifting u64 by 64 is UB
    // } else {
    //     (1u64 << blocks) - 1 // for 3kb mask would be 0b111
    // };

    critical_section::with(|cs| {
        let pool = unsafe {&mut *MEMORY.borrow(cs).get() };

        if let Some(start_idx) = pool.bitmap.find_first_fit(blocks) {
            pool.bitmap.set_range(start_idx, blocks); // found, mark as used

            let base_ptr = pool.buffer.as_mut_ptr() as *mut u8;
            let start_offset = start_idx * BLOCK_SIZE;
            let alloc_ptr = unsafe { base_ptr.add(start_offset) };

            let size = blocks * BLOCK_SIZE;
            return Some(MemChunk { ptr: alloc_ptr, size});
        }
        None
    })
}

fn dealloc(chunk: &mut MemChunk) {
    critical_section::with(|cs| {
        let pool = unsafe { &mut *MEMORY.borrow(cs).get() };
        let base_ptr = pool.buffer.as_mut_ptr() as *mut u8;

        let offset = chunk.ptr as usize - base_ptr as usize; // todo: wrapping sub?
        let start_bit = offset / BLOCK_SIZE;

        let blocks = chunk.size.div_ceil(BLOCK_SIZE);

        pool.bitmap.clear_range(start_bit, blocks);
    })
}

impl Drop for MemChunk {
    fn drop(&mut self) {
        dealloc(self);
    }
}