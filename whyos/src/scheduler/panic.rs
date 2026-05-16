use core::panic::PanicInfo;
use crate::yield_cpu;
use crate::scheduler::Kernel;
use crate::utils::log;

use crate::arch::{is_in_task, bkpt, wfi};

#[inline(never)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let is_task = is_in_task();

    if is_task {
        if let Some(tid) = Kernel::try_lock(|k| k.current_task()).flatten() {
            log::warn!("WhyOS: Task {} panicked: {}", tid.id(), info);

            let _ = Kernel::lock(|k| k.make_zombie(tid));
            yield_cpu();

            loop {
                wfi();
            }
        } else {
            log::error!("WhyOS: Idle task panic: {}", info);
            loop {
                bkpt();
            }
        }
    } else {
        log::error!("WhyOS: KERNEL PANIC: {}", info);
        loop {
            bkpt();
        }
    }
}