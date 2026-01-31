use core::{arch::naked_asm};
use cortex_m_rt::ExceptionFrame;

use crate::{TaskId, TaskInfo};
use crate::syscall::{self, SvcNumber};
use crate::error::{ErrNo, SUCCESS};

#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn SVCall() { // todo: probably can be optimised
    naked_asm!(
        // get stack pointer
        "tst lr, #4", // test bit 2 of LR
        "ite eq",
        "mrseq r0, msp", // if handler mode: r0 = MSP
        "mrsne r0, psp", // if thread mode: r0 = PSP

        // extract SVC number
        "ldr r1, [r0, #24]", // get program counter from exception frame, at offset 24
        "ldrb r1, [r1, #-2]", // get svc immediate (bottom 8bits) - offset "-2" because PC already advanced

        // check if we bootstraping the scheduler
        "cmp r1, #0",
        "beq bootstrap", // branch to bootstrap path

        // --- SYSCALL PATH ---
        "push {{lr}}", // save LR and r0 (containing stack pointer) before calling function

        // call dispatch (frame_ptr, svc_num)
        "bl svc_dispatch", // r0 contains the return value

        // restore LR
        "pop {{lr}}",

        "bx lr", // return to whatever mode we were in

        // --- BOOTSTRAP PATH ---
        "bootstrap:",

        // security check - tasks shouldn't call bootstrap
        "tst lr, #4",
        "bne reject_bootstrap",

        // Setup PSP
        "bl get_idle_task_sp", // returns idle sp in r0
        "ldmia r0!, {{r4-r11, lr}}", // discard "fake" sw frame built during initialization and update r0, load default LR
        "msr psp, r0", // set sp to r0 (hw frame)
        "isb", // flushes cpu pipeline, needed because we overwritten stack pointer

        "bx lr", // start os in PSP mode


        // --- BOOTSTRAP REJECTION ---
        "reject_bootstrap:",
        "b reject_bootstrap", // todo: return error instead of infinite loop

        "bx lr", // dead code for now
    );
}

#[unsafe(no_mangle)]
extern "C" fn svc_dispatch(ef: &mut ExceptionFrame, svc_id: u8) {
    let Ok(svc) = SvcNumber::try_from(svc_id) else {
        unsafe { ef.set_r0(crate::error::WhyError::InvalidOperation as u32); }
        return;
    };

    use SvcNumber as SVC;
    match svc {
        SVC::Start => panic!("BOOTSTRAP REJECTION FAILED"),
        SVC::Yield => { syscall::yield_now(); } // PenSV has lower prio, it will execute once we return from SVC
        SVC::Sleep => {
            let ticks = (ef.r0() as u64) | ((ef.r1() as u64) << 32);
            syscall::sleep(ticks);
        },
        SVC::Exit => {
            syscall::exit();
        },
        SVC::Suspend => {
            let tid = TaskId(ef.r0() as usize);
            unsafe { ef.set_r0(syscall::suspend(tid).to_errno() as u32) };
        },
        SVC::Resume => {
            let tid = TaskId(ef.r0() as usize);
            unsafe { ef.set_r0(syscall::resume(tid).to_errno() as u32) };
        },
        SVC::GetCurrentTid => {
            unsafe { ef.set_r0(syscall::get_current_tid().0 as u32) };
        },
        SVC::GetCurrentName => {
            match syscall::get_current_name() {
                Some(name) => unsafe {
                    ef.set_r0(name.as_ptr() as u32);
                    ef.set_r1(name.len() as u32);
                },
                None => unsafe {
                    ef.set_r0(0);
                    ef.set_r1(0);
                }
            }
        },
        SVC::GetUptimeTicks => {
            let ticks = syscall::get_uptime_ticks();
            unsafe {
                ef.set_r0(ticks as u32);
                ef.set_r1((ticks >> 32) as u32);
            }
        },
        SVC::GetTaskCount => {
            unsafe { ef.set_r0(syscall::get_task_count() as u32) };
        }
        SVC::GetTaskInfo => {
            let tid = TaskId(ef.r0() as usize);
            let task_info_ptr = ef.r1() as *mut TaskInfo;

            match syscall::get_task_info(tid) {
                Ok(info) => unsafe {
                    task_info_ptr.write(info);
                    ef.set_r0(SUCCESS as u32);
                },
                Err(e) => unsafe { ef.set_r0(e as u32) },
            }
        },
        SVC::GetActiveTasks => {
            let allocated_map = syscall::get_allocated_tasks();
            unsafe { ef.set_r0(allocated_map.0) };
        },
        SVC::ReclaimMemory => {
            let reclaimed = syscall::reclaim_memory();
            unsafe { ef.set_r0(reclaimed as u32) };
        },
        SVC::WatchdogSubscribe => {
            let interval = (ef.r0() as u64) | ((ef.r1() as u64) << 32);
            syscall::watchdog_subscribe(interval);
        },
        SVC::WatchdogUnsubscribe => {
            syscall::watchdog_unsubscribe();
        },
        SVC::WatchdogFeed => {
            syscall::watchdog_feed();
        },
        SVC::Spawn => {
            let args_ptr = ef.r0() as *const crate::syscall::SpawnArgs;
            let args = unsafe { &*args_ptr };

            let entry = unsafe { core::mem::transmute::<usize, crate::task::TaskEntryPoint>(args.entry) };
            let name = if args.name_ptr.is_null() {
                None
            } else {
                Some(unsafe {
                    core::str::from_utf8_unchecked(
                        core::slice::from_raw_parts(args.name_ptr, args.name_len)
                    )
                })
            };

            match crate::task::ops::spawn(entry, args.arg, name, args.priority, args.stack_size) {
                Ok(tid) => unsafe {
                    ef.set_r0(SUCCESS as u32);
                    ef.set_r1(tid.0 as u32);
                },
                Err(e) => unsafe {
                    ef.set_r0(e as u32);
                }
            }
        }
    }
}