use core::{arch::naked_asm};
use cortex_m_rt::ExceptionFrame;

use crate::TaskId;
use crate::syscall::{self, SvcNumber};
use crate::error::ErrNo;

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
        "push {{r0, lr}}", // save LR and r0 (containing stack pointer) before calling function

        // call dispatch (frame_ptr, svc_num)
        "bl svc_dispatch", // r0 contains the return value

        // restore LR
        "pop {{r2, lr}}",

        "str r0, [r2, #0]", // return value, we overwrite the r0 slot in exception frame (slot 0)

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
extern "C" fn svc_dispatch(ef: &ExceptionFrame, svc_id: SvcNumber) -> usize {
    let mut ret_val: usize = 0;

    use SvcNumber as SVC;
    match svc_id {
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
            ret_val = syscall::suspend(tid).to_errno();
        },
        SVC::Resume => {
            let tid = TaskId(ef.r0() as usize);
            ret_val = syscall::resume(tid).to_errno();
        },
        SVC::GetCurrentTid => {
            ret_val = syscall::get_current_tid().0;
        }
        _ => todo!()
    }

    ret_val
}