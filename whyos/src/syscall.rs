use core::num::NonZero;

use crate::arch;
use crate::error::WhyResult;
use crate::scheduler::{self, Kernel};
use crate::task::TaskId;
use crate::{TaskBuilder, TaskHandle, TaskRoutine};
use crate::utils::log;

/// Represents a frequency used to configure the kernel's system tick timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Freq(u32); // Hz

impl Freq {
    /// A convenience constant representing 1 kilohertz (1000 Hz).
    pub const ONE_KHZ: Self = Self(1000);

    /// Attempts to create new `Freq` from specified Hertz, returning `None` if `hz` is 0.
    #[inline]
    pub const fn from_hz(hz: u32) -> Option<Self> {
        if hz == 0 {
            None
        } else {
            Some(Self(hz))
        }
    }

    /// Attempts to create new `Freq` from specified Kilohertz, returning `None` if `khz` is 0.
    ///
    /// The resulting frequency is saturating if the calculation overflows the maximum `u32` value.
    #[inline]
    pub const fn from_khz(khz: u32) -> Option<Self> {
        Self::from_hz(khz.saturating_mul(1_000))
    }

    /// Returns the raw frequency value in Hertz.
    #[inline]
    pub const fn as_hz(self) -> u32 {
        self.0
    }
}

/// Initializes and starts the WhyOS kernel at the given tick frequency.
///
/// This sets up kernel scheduling and then transfers control to the target
/// architecture startup routine. It does not return.
///
/// # Panics
/// Panics if called more than once to avoid Kernel corruption.
pub fn start(freq: Freq) -> ! {
    log::info!("Starting WhyOS at {} Hz", freq.as_hz());
    log::debug!("WhyOS build ident: {}", build_name());
    Kernel::init(freq.as_hz());
    unsafe { arch::start_os(freq.as_hz()) }
}

/// Changes the kernel tick frequency at runtime.
///
/// This updates the scheduler interval and forwards the new rate to the target
/// architecture timer driver.
pub fn set_tick_freq(freq: Freq) {
    use crate::arch::KernelArch;
    log::debug!("Set tick freq to {} Hz", freq.as_hz());
    Kernel::lock(|k| k.set_timer_interval(freq.as_hz()));
    arch::TargetArch::set_tick_freq(freq.as_hz());
}

/// Returns the currently active kernel tick frequency in hertz.
pub fn tick_freq() -> u32 {
    Kernel::lock(|k| k.timer_interval())
}

/// Convenience wrapper around [`TaskBuilder::spawn`].
#[inline]
pub fn spawn(entry: TaskRoutine) -> WhyResult<TaskHandle> { TaskBuilder::new(entry).spawn() }

/// Convenience wrapper around [`TaskBuilder::priority`] and [`TaskBuilder::spawn`].
#[inline]
pub fn spawn_with_priority(entry: TaskRoutine, priority: u8) -> WhyResult<TaskHandle> { TaskBuilder::new(entry).priority(priority).spawn() }

/// Yields the current task to the scheduler.
#[inline]
pub fn yield_cpu() {
    log::trace!("manual yield");
    scheduler::yield_now();
}

/// Puts the current task to sleep for at least the given number of ticks.
///
/// A value of `0` is ignored and the task simply yields.
#[inline]
pub fn sleep(ticks: u64) {
    if let Some(ticks) = NonZero::new(ticks) {
        let curr = current_tid();
        log::debug!("Sleep task {} ticks {}", curr.id(), ticks.get());
        Kernel::lock(|k| {
            k.sleep_task(curr, ticks);
        });
    }

    scheduler::yield_now();
}

/// Terminates the current task and marks it for cleanup,
///
/// **WARNING:** This function does not run destructors. If the task holds a
/// locked [`crate::itc::Mutex`] or other shared resources, they remain locked forever.
#[inline]
pub fn exit() -> ! {
    let curr = current_tid();
    log::info!("Task {} exit", curr.id());
    Kernel::lock(|k| k.make_zombie(curr));
    scheduler::yield_now();
    loop {}
}

/// Returns the task id of the currently running task.
///
/// # Panics
/// Panics if called outside task context.
#[inline]
pub(crate) fn current_tid() -> TaskId {
    Kernel::lock(|k| k.current_task()
        .expect("WhyOS: No current task")
    )
}

/// Returns a handle for the currently running task.
///
/// # Panics
/// Panics if called outside task context or if the current task is invalid.
pub fn my_handle() -> TaskHandle {
    let curr = current_tid();
    Kernel::lock(|k| {
        k.handle(curr)
            .expect("WhyOS: Current task invalid")
    })
}

/// Returns the number of system ticks since boot.
#[inline]
pub fn uptime_ticks() -> u64 {
    Kernel::lock(|k| k.system_ticks())
}

/// Returns an iterator over all allocated task handles.
#[inline]
pub fn allocated() -> impl Iterator<Item = TaskHandle> {
    let map = Kernel::lock(|k| k.allocated());
    map.iter().filter_map(|tid| Kernel::lock(|k| k.handle(tid).ok()))
}

/// Reaps zombie tasks and returns their memory to the allocator.
#[inline]
pub fn reclaim_memory() {
    Kernel::lock(|k| k.reap_zombies())
}

/// Subscribes the current task to the watchdog with the given interval.
///
/// A value of `0` is ignored.
#[inline]
pub fn wdt_sub(interval_ticks: u64) {
    if let Some(ticks) = NonZero::new(interval_ticks) {
        let curr = current_tid();
        log::debug!("Task {} wdt subscribe", curr.id());
        Kernel::lock(|k| {
            k.wdt_sub(curr, ticks);
        })
    }
}

/// Unsubscribes the current task from the watchdog.
#[inline]
pub fn wdt_unsub() {
    let curr = current_tid();
    Kernel::lock(|k| {
        log::debug!("Task {} wdt unsubscribe", curr.id());
        k.wdt_unsub(curr);
    })
}

/// Feeds the watchdog for the current task.
#[inline]
pub fn wdt_feed() {
    let curr = current_tid();
    Kernel::lock(|k| {
        log::trace!("Task {} wdt feed", curr.id());
        k.wdt_feed(curr);
    })
}

/// Reboots the system.
///
/// This does not return.
#[inline]
pub fn reboot() -> ! {
    log::info!("Performing reboot");
    arch::reset();
}

/// Returns the build identifier embedded at compile time.
#[inline]
pub const fn build_name() -> &'static str {
    env!("WHYOS_BUILD_IDENT")
}