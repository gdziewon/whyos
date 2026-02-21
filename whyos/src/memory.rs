use core::{cell::UnsafeCell, mem::MaybeUninit};

use critical_section::Mutex;


const POOL_SIZE: usize = 64;
const BLOCK_SIZE: usize = 1024; // 1kb
const TOTAL_BYTES: usize = POOL_SIZE * BLOCK_SIZE; // 64kb

#[repr(C, align(8))]
struct MemoryPool {
    buffer: MaybeUninit<[u8; TOTAL_BYTES]>,
    bitmap: u64
}

unsafe impl Sync for MemoryPool {}

// simple bitmap allocator
static MEMORY: Mutex<UnsafeCell<MemoryPool>> = Mutex::new(UnsafeCell::new(MemoryPool {
    buffer: MaybeUninit::uninit(),
    bitmap: 0,
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

    let search_mask: u64 = if blocks == 64 {
        u64::MAX // to handle edge case of 64 - shifting u64 by 64 is UB
    } else {
        (1u64 << blocks) - 1 // for 3kb mask would be 0b111
    };

    critical_section::with(|cs| {
        let pool = unsafe {&mut *MEMORY.borrow(cs).get() };


        // FIRST-FIT
        for i in 0..=(POOL_SIZE - blocks) {

            if (pool.bitmap & (search_mask << i)) == 0 { // check if all are 0 (maaaybe can be optimized by checking which one was not free last)
                pool.bitmap |= search_mask << i; // found one, mark bits as used

                let base_ptr = pool.buffer.as_mut_ptr() as *mut u8;
                let start_offset = i * BLOCK_SIZE;
                let alloc_ptr = unsafe { base_ptr.add(start_offset) };

                let size = blocks * BLOCK_SIZE;
                return Some(MemChunk { ptr: alloc_ptr, size});
            }
        }
        None
    })
}

pub unsafe fn dealloc(chunk: MemChunk) {
    critical_section::with(|cs| {
        let pool = unsafe { &mut *MEMORY.borrow(cs).get() };
        let base_ptr = pool.buffer.as_mut_ptr() as *mut u8;

        let offset = chunk.ptr as usize - base_ptr as usize;
        let start_bit = offset / BLOCK_SIZE;

        let blocks = chunk.size.div_ceil(BLOCK_SIZE);

        let mask = if blocks == POOL_SIZE {
            u64::MAX
        } else {
            (1u64 << blocks) - 1
        };

        pool.bitmap &= !(mask << start_bit);
    })
}