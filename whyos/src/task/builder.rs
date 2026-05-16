use core::mem;

use super::stack;
use crate::{error::WhyResult, task::TaskHandle};

pub type TaskRoutine = extern "C" fn();
pub type TaskRoutineArg<T> = extern "C" fn(T);

pub struct TaskBuilder {
    entry: stack::TaskEntryPoint,
    arg: usize,
    priority: u8,
    stack_size: usize,
    name: Option<&'static str>
}

impl TaskBuilder {
    #[inline(always)]
    fn from_raw(entry: stack::TaskEntryPoint, arg: usize) -> Self {
        Self {
            entry,
            arg,
            priority: 128,
            stack_size: StackSize::DEFAULT.0,
            name: None,
        }
    }

    #[inline]
    pub fn new(entry: TaskRoutine) -> Self {
        // cast fn() to fn(usize)
        // when scheduler runs this, it will put garbage in r0 register
        // but the function entry takes no arguments, so it will ignore r0 anyway
        let entry_transmuted = unsafe { mem::transmute::<TaskRoutine, stack::TaskEntryPoint>(entry) };

        Self::from_raw(entry_transmuted, 0)
    }

    #[inline]
    pub fn with_value<T>(
        entry: TaskRoutineArg<T>,
        arg: T
    ) -> Self
    where
        T: Copy + Send + 'static
    {
        // it needs to fit into r0, which is only 32bits (usize)
        const { assert!(
            mem::size_of::<T>() <= mem::size_of::<usize>(),
            "Argument too large to pass by value"
        )};

        // bit-packing T into usize, we already know it fits
        let mut arg_storage: usize = 0;
        unsafe {
            ((&mut arg_storage) as *mut usize as *mut T).write(arg);
        }

        // cast fn(T) to fn(usize)
        // we confirmed that T is both Copy, Send and static
        // and that it fits inside usize
        let entry_transmuted = unsafe { mem::transmute::<TaskRoutineArg<_>, stack::TaskEntryPoint>(entry) };

        Self::from_raw(entry_transmuted, arg_storage)
    }

    #[inline]
    pub fn with_static_mut<T>(
        entry: TaskRoutineArg<&'static mut T>,
        arg: &'static mut T
    ) -> Self
    where
        T: Send + 'static,
    {
        // cast fn(&'static mut T) to fn(usize)
        // reference is just a pointer under the hood
        let entry_transmuted = unsafe { mem::transmute::<TaskRoutineArg<_>, stack::TaskEntryPoint>(entry) };

        let arg_addr = arg as *mut T as usize;

        Self::from_raw(entry_transmuted, arg_addr)
    }

    #[inline]
    pub fn with_static_ref<T>(
        entry: TaskRoutineArg<&'static T>,
        arg: &'static T
    ) -> Self
    where
        T: Sync + 'static,
    {
        // cast fn(&'static T) to fn(usize)
        // again, reference is just a pointer under the hood
        let entry_transmuted = unsafe { mem::transmute::<TaskRoutineArg<_>, stack::TaskEntryPoint>(entry) };

        let arg_addr = arg as *const T as usize;
        Self::from_raw(entry_transmuted, arg_addr)
    }

    // unsafe because ptrs arent Send and 'validness' is unchecked
    #[inline]
    pub unsafe fn with_ptr_mut<T>(
        entry: TaskRoutineArg<*mut T>,
        arg: *mut T
    ) -> Self {
        // cast fn(*mut T) to fn(usize)
        // function takes pointer, we cast it to pointer-sized-taking function
        let entry_transmuted = unsafe { mem::transmute::<TaskRoutineArg<_>, stack::TaskEntryPoint>(entry) };

        let arg_addr = arg as usize;
        Self::from_raw(entry_transmuted, arg_addr)
    }

    // unsafe, same as above
    #[inline]
    pub unsafe fn with_ptr_const<T>(
        entry: TaskRoutineArg<*const T>,
        arg: *const T
    ) -> Self {
        // cast fn(*const T) to fn(usize)
        // same as above
        let entry_transmuted = unsafe { mem::transmute::<TaskRoutineArg<_>, stack::TaskEntryPoint>(entry) };

        let arg_addr = arg as usize;
        Self::from_raw(entry_transmuted, arg_addr)
    }

    #[inline]
    pub fn priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    #[inline]
    pub fn stack_size(mut self, size: StackSize) -> Self {
        self.stack_size = size.0;
        self
    }

    #[inline]
    pub fn name(mut self, name: &'static str) -> Self {
        self.name = Some(name);
        self
    }

    #[inline]
    pub fn spawn(self) -> WhyResult<TaskHandle> {
        super::spawn(self.entry, self.arg, self.name, self.priority, self.stack_size)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StackSize(usize);

impl StackSize {
    pub const SMALL: Self = Self(1024); // 1kb

    pub const DEFAULT: Self = Self(2048); // 2kb

    pub const MEDIUM: Self = Self(4096); // 4kb

    pub const LARGE: Self = Self(8192); // 8kb

    #[inline]
    pub const fn bytes(bytes: usize) -> Self {
        Self(bytes)
    }

    #[inline]
    pub const fn kb(kb: usize) -> Self {
        Self(kb * 1024)
    }

    #[inline]
    pub const fn as_bytes(&self) -> usize {
        self.0
    }
}

impl Default for StackSize {
    #[inline]
    fn default() -> Self {
        Self::DEFAULT
    }
}