use core::num::NonZero;

use crate::arch;
use crate::error::WhyResult;
use crate::scheduler::{self, Kernel};
use crate::task::TaskId;
use crate::{TaskBuilder, TaskHandle, TaskRoutine};
use crate::utils::log;

// TODO: Add docs

/// Represents a frequency, internally stored in Hertz (Hz).
///
/// This is typically used to configure the kernel's system tick timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Freq(u32); // Hz

impl Freq {
    /// A convenience constant representing 1 kilohertz (1000 Hz).
    pub const ONE_KHZ: Self = Self(1000);

    /// Creates a Frequency from Hertz. Returns `None` if `hz` is 0.
    #[inline]
    pub const fn from_hz(hz: u32) -> Option<Self> {
        if hz == 0 {
            None
        } else {
            Some(Self(hz))
        }
    }

    /// Creates a Frequency from Kilohertz. Returns `None` if `khz` is 0.
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

/// Initializes and starts the WhyOS kernel and begins task scheduling
/// at given frequency.
///
/// # Panics
/// This function enforces strict one-shot initialization. It will `panic!` if
/// called more than once to prevent hardware and kernel state corruption.
pub fn start(freq: Freq) -> ! {
    log::info!("Starting WhyOS at {} Hz", freq.as_hz());
    log::debug!("WhyOS build ident: {}", build_name());
    Kernel::init(freq.as_hz());
    unsafe { arch::start_os(freq.as_hz()) }
}

/// Changes the system tick frequency at runtime.
///
/// This will dynamically update the internal scheduling frequency.
pub fn set_tick_freq(freq: Freq) {
    use crate::arch::KernelArch;
    log::debug!("Set tick freq to {} Hz", freq.as_hz());
    Kernel::lock(|k| k.set_timer_interval(freq.as_hz()));
    arch::TargetArch::set_tick_freq(freq.as_hz());
}

/// Returns the currently active system tick frequency.
pub fn tick_freq() -> u32 {
    Kernel::lock(|k| k.timer_interval())
}

#[inline] pub fn spawn(entry: TaskRoutine) -> WhyResult<TaskHandle> { TaskBuilder::new(entry).spawn() }
#[inline] pub fn spawn_with_priority(entry: TaskRoutine, priority: u8) -> WhyResult<TaskHandle> { TaskBuilder::new(entry).priority(priority).spawn() }

#[inline]
pub fn yield_cpu() {
    log::trace!("manual yield");
    scheduler::yield_now();
}

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

/// Terminates the current task and reclaims its memory.
///
/// **WARNING:** This function does NOT run destructors. If the task
/// holds a locked `Mutex` or other shared resources, they will remain
/// locked forever.
#[inline]
pub fn exit() -> ! {
    let curr = current_tid();
    log::info!("Task {} exit", curr.id());
    Kernel::lock(|k| k.make_zombie(curr));
    scheduler::yield_now();
    loop {}
}

#[inline]
pub(crate) fn current_tid() -> TaskId {
    Kernel::lock(|k| k.current_task()
        .expect("WhyOS: No current task")
    )
}

pub fn my_handle() -> TaskHandle {
    let curr = current_tid();
    Kernel::lock(|k| {
        k.handle(curr)
            .expect("WhyOS: Current task invalid")
    })
}

#[inline]
pub fn uptime_ticks() -> u64 {
    Kernel::lock(|k| k.system_ticks())
}

#[inline]
pub fn allocated() -> impl Iterator<Item = TaskHandle> {
    let map = Kernel::lock(|k| k.allocated());
    map.iter().filter_map(|tid| Kernel::lock(|k| k.handle(tid).ok()))
}

#[inline]
pub fn reclaim_memory() {
    Kernel::lock(|k| k.reap_zombies())
}

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

#[inline]
pub fn wdt_unsub() {
    let curr = current_tid();
    Kernel::lock(|k| {
        log::debug!("Task {} wdt unsubscribe", curr.id());
        k.wdt_unsub(curr);
    })
}

#[inline]
pub fn wdt_feed() {
    let curr = current_tid();
    Kernel::lock(|k| {
        log::trace!("Task {} wdt feed", curr.id());
        k.wdt_feed(curr);
    })
}

#[inline]
pub fn reboot() -> ! {
    log::info!("Performing reboot");
    arch::reset();
}

#[inline]
pub const fn build_name() -> &'static str {
    env!("WHYOS_BUILD_IDENT")
}