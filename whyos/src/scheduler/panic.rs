use core::panic::PanicInfo;
use crate::yield_cpu;
use crate::scheduler::Kernel;

use crate::arch::{is_in_task, bkpt, wfi};

#[inline(never)]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let is_task = is_in_task();

    if is_task {
        if let Some(tid) = Kernel::lock(|k| k.current_task()) {
            defmt::warn!("WhyOS: Task {} panicked: {}", tid.id(), info);

            let _ = Kernel::lock(|k| k.make_zombie(tid));
            yield_cpu();

            loop {
                wfi();
            }
        } else {
            defmt::error!("WhyOS: Idle task panic: {}", info);
            loop {
                bkpt();
            }
        }
    } else {
        defmt::error!("WhyOS: KERNEL PANIC: {}", info);
        loop {
            bkpt();
        }
    }
}