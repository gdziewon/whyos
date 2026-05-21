use core::mem;

use super::stack;
use crate::{error::WhyResult, task::TaskHandle};

/// A parameterless task entry point.
pub type TaskRoutine = extern "C" fn();

/// A task entry point that takes a single argument of type `T`.
pub type TaskRoutineArg<T> = extern "C" fn(T);

/// A builder for configuring and spawning new tasks in the OS.
///
/// This builder allows you to set the task's entry point, argument, priority,
/// stack size, and an optional name before submitting it to the scheduler.
///
/// # Default Values
/// * **Priority**: `128`
/// * **Stack Size**: [`StackSize::DEFAULT`]
/// * **Name**: `None`
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

    /// Creates a new `TaskBuilder` for a task that takes no arguments.
    #[inline]
    pub fn new(entry: TaskRoutine) -> Self {
        // cast fn() to fn(usize)
        // when scheduler runs this, it will put garbage in r0 register
        // but the function entry takes no arguments, so it will ignore r0 anyway
        let entry_transmuted = unsafe { mem::transmute::<TaskRoutine, stack::TaskEntryPoint>(entry) };

        Self::from_raw(entry_transmuted, 0)
    }

    /// Creates a new `TaskBuilder` passing an argument by value.
    ///
    /// The argument type `T` must fit inside a single machine word.
    /// This is typically used for passing integers, booleans, or small enums.
    ///
    /// # Panics
    ///
    /// This function will fail to compile if the size of `T` is greater than the
    /// size of a pointer (`usize`).
    #[inline]
    pub fn with_value<T>( // todo: remove all of those "with_" - leave only usize
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

    /// Creates a new `TaskBuilder` passing a mutable static reference.
    ///
    /// The data being referenced must live for the entire lifetime of the OS (`'static`)
    /// and must be safe to send across task boundaries (`Send`).
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

    /// Creates a new `TaskBuilder` passing an immutable static reference.
    ///
    /// The data being referenced must live for the entire lifetime of the OS (`'static`)
    /// and must be safe to share across task boundaries (`Sync`).
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

    /// Creates a new `TaskBuilder` passing a raw mutable pointer.
    ///
    /// # Safety
    ///
    /// Raw pointers bypass Rust's thread-safety (`Send`/`Sync`) and lifetime checks.
    /// The caller must guarantee that:
    /// 1. The pointer remains valid for the duration of the task's execution.
    /// 2. Concurrent access to the pointed data is properly synchronized if necessary.
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

    /// Creates a new `TaskBuilder` passing a raw constant pointer.
    ///
    /// # Safety
    ///
    /// Raw pointers bypass Rust's thread-safety (`Send`/`Sync`) and lifetime checks.
    /// The caller must guarantee that:
    /// 1. The pointer remains valid for the duration of the task's execution.
    /// 2. The memory pointed to is safely accessible by the spawned task.
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

    /// Sets the priority of the task.
    ///
    /// Lower values represent higher scheduling priorities.
    #[inline]
    pub fn priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Sets the stack size allocated for the task.
    ///
    /// See [`StackSize`] for predefined sensible defaults.
    #[inline]
    pub fn stack_size(mut self, size: StackSize) -> Self {
        self.stack_size = size.0;
        self
    }

    /// Assigns an optional string name to the task.
    ///
    /// Task names are highly recommended as they greatly simplify debugging
    /// and profiling via the OS shell or logs.
    #[inline]
    pub fn name(mut self, name: &'static str) -> Self {
        self.name = Some(name);
        self
    }

    /// Submits the configured task to the scheduler, returning `Ok(TaskHandle)` if successful.
    #[inline]
    pub fn spawn(self) -> WhyResult<TaskHandle> {
        super::spawn(self.entry, self.arg, self.name, self.priority, self.stack_size)
    }
}


/// Representation of tasks stack size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StackSize(usize);

impl StackSize {
    /// A small stack size of 1 KB.
    pub const SMALL: Self = Self(1024);

    /// The default stack size of 2 KB.
    pub const DEFAULT: Self = Self(2048);

    /// A medium stack size of 4 KB.
    pub const MEDIUM: Self = Self(4096);

    /// A large stack size of 8 KB.
    pub const LARGE: Self = Self(8192);

    /// Constructs a `StackSize` from an exact byte count.
    ///
    /// Depending on allocator implementation, this might be rounded up to block size.
    #[inline]
    pub const fn bytes(bytes: usize) -> Self {
        Self(bytes)
    }

    /// Constructs a `StackSize` from kilobytes (e.g., `kb(2)` equals 2048 bytes).
    #[inline]
    pub const fn kb(kb: usize) -> Self {
        Self(kb * 1024)
    }

    /// Returns the stack size in bytes as a raw `usize`.
    #[inline]
    pub const fn as_bytes(&self) -> usize {
        self.0
    }
}

impl Default for StackSize {
    /// Defaults to `StackSize::DEFAULT`.
    #[inline]
    fn default() -> Self {
        Self::DEFAULT
    }
}