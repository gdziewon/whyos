use core::mem::MaybeUninit;

use crate::{StackSize, memory::StaticMemory, task::{Stack, task_exit_trampoline}};

const IDLE_STACK_SIZE: usize = StackSize::SMALL.as_bytes();

#[repr(C, align(8))]
struct IdleMemory(MaybeUninit<[u8; IDLE_STACK_SIZE]>);

static mut IDLE_MEMORY: IdleMemory = IdleMemory(MaybeUninit::uninit());

extern "C" fn idle_entry(_: usize) {
    loop {
        crate::task::ops::reap_zombies();
        crate::arch::wfi();
    }
}

pub(crate) struct IdleTask {
    stack: Stack<StaticMemory>,
}

impl IdleTask {
    pub fn new() -> Self {
        let return_handler = task_exit_trampoline as *const () as usize;
        let mem = unsafe {
            let ptr = core::ptr::addr_of_mut!(IDLE_MEMORY) as *mut u8;
            StaticMemory::from_raw(ptr, IDLE_STACK_SIZE)
        };
        let stack = Stack::init(mem, idle_entry, 0, return_handler);
        Self { stack }
    }

    #[inline]
    pub fn sp(&self) -> usize {
        self.stack.sp()
    }

    #[inline]
    pub fn set_sp(&mut self, sp: usize) {
        self.stack.set_sp(sp);
    }
}