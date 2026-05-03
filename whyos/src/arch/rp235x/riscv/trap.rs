use core::arch::naked_asm;
use crate::scheduler::Kernel;
use super::frame::InitStackFrame;
use super::start::SioTimer;

//https://www2.eecs.berkeley.edu/Pubs/TechRpts/2016/EECS-2016-161.pdf#page=46

#[derive(Clone, Copy)]
pub enum TrapCause {
    Interrupt(Interrupt),
    Exception(Exception),
}

#[derive(Clone, Copy, PartialEq)]
pub enum Interrupt {
    MachineSoftware,
    MachineTimer,
    MachineExternal,
    Unknown(u32),
}

#[allow(dead_code)]
#[derive(defmt::Format, Clone, Copy, PartialEq)]
pub enum Exception {
    InstructionAddressMisaligned,
    InstructionAccessFault,
    IllegalInstruction,
    Breakpoint,
    LoadAddressMisaligned,
    LoadAccessFault,
    StoreAddressMisaligned,
    StoreAccessFault,
    EnvironmentCallFromUMode,
    EnvironmentCallFromSMode,
    EnvironmentCallFromMMode,
    InstructionPageFault,
    LoadPageFault,
    StorePageFault,
    Unknown(u32),
}

impl TrapCause {
    const INTERRUPT_BIT: u32 = 31;

    pub fn from_mcause(mcause: u32) -> Self {
        let is_interrupt = (mcause >> Self::INTERRUPT_BIT) != 0;
        let code = mcause & 0x7FFF_FFFF;

        if is_interrupt {
            TrapCause::Interrupt(match code {
                3 => Interrupt::MachineSoftware,
                7 => Interrupt::MachineTimer,
                11 => Interrupt::MachineExternal,
                _ => Interrupt::Unknown(code),
            })
        } else {
            TrapCause::Exception(match code {
                2 => Exception::IllegalInstruction,
                3 => Exception::Breakpoint,
                11 => Exception::EnvironmentCallFromMMode,
                _ => Exception::Unknown(code),
            })
        }
    }
}

unsafe extern "C" {
    fn MachineExternal();
}

#[unsafe(no_mangle)]
extern "C" fn trap_handler(sp: usize, mcause: u32, mtval: u32) -> usize {
    let frame = unsafe { &mut *(sp as *mut InitStackFrame) };
    let cause = TrapCause::from_mcause(mcause);

    match cause {
        TrapCause::Interrupt(Interrupt::MachineTimer) => {
            Kernel::lock(|k| {
                let interval = k.timer_interval() as u64;

                let now = SioTimer::get_time();
                SioTimer::set_compare(now + interval);

                k.on_tick();
                k.schedule(sp)
            })
        },
        TrapCause::Interrupt(Interrupt::MachineExternal) => {
            unsafe { MachineExternal(); }
            Kernel::lock(|k| k.schedule(sp))
        },
        TrapCause::Interrupt(_) => { // MachineSoftware (calling ecall) and everything else
            Kernel::lock(|k| k.schedule(sp))
        },
        TrapCause::Exception(Exception::EnvironmentCallFromMMode) => {
            // in case of exception, mepc has the exact intruction that caused exception
            // in case of interrupts, it has "next instruction"
            // so for exception we gotta jump to this next instruction, 4bytes
            frame.mepc += 4;
            Kernel::lock(|k| k.schedule(sp))
        },
        TrapCause::Exception(Exception::Breakpoint) => {
            frame.mepc += 4;
            sp // its a breakpoint, dont context switchx
        }
        TrapCause::Exception(e) => {
            defmt::error!("WhyOS: FATAL: exc: {}, mepc: {:X}, mtval: {:X}", e, frame.mepc, mtval);
            crate::arch::bkpt();
            loop { crate::arch::wfi(); }
        }
    }
}

#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn trap_entry() {
    naked_asm!(
        ".align 4",
        "addi sp, sp, -128", // move sp down 128 bytes (32regs X 4bytes)

        // push regs
        "sw x1, 0(sp)",
        "sw x3, 4(sp)", // skipping x0 (zero reg) and x2
        "sw x4, 8(sp)",
        "sw x5, 12(sp)",
        "sw x6, 16(sp)",
        "sw x7, 20(sp)",
        "sw x8, 24(sp)",
        "sw x9, 28(sp)",
        "sw x10, 32(sp)",
        "sw x11, 36(sp)",
        "sw x12, 40(sp)",
        "sw x13, 44(sp)",
        "sw x14, 48(sp)",
        "sw x15, 52(sp)",
        "sw x16, 56(sp)",
        "sw x17, 60(sp)",
        "sw x18, 64(sp)",
        "sw x19, 68(sp)",
        "sw x20, 72(sp)",
        "sw x21, 76(sp)",
        "sw x22, 80(sp)",
        "sw x23, 84(sp)",
        "sw x24, 88(sp)",
        "sw x25, 92(sp)",
        "sw x26, 96(sp)",
        "sw x27, 100(sp)",
        "sw x28, 104(sp)",
        "sw x29, 108(sp)",
        "sw x30, 112(sp)",
        "sw x31, 116(sp)",

        // pc
        "csrr t0, mepc",
        "sw t0, 120(sp)",
        // mstatus
        "csrr t0, mstatus",
        "sw t0, 124(sp)",

        // args a0-3 for trap handler
        "mv a0, sp",
        "csrr a1, mcause",
        "csrr a2, mtval",

        "call trap_handler", // a0 will have new sp

        "mv sp, a0", // set new sp

        ".globl restore_context", // so we dont repeat ourselves in start
        "restore_context:",

        // new mstatus
        "lw t0, 124(sp)",
        "csrw mstatus, t0",
        // new pc
        "lw t0, 120(sp)",
        "csrw mepc, t0",

        // pop new regs
        "lw x1, 0(sp)",
        "lw x3, 4(sp)",
        "lw x4, 8(sp)",
        "lw x5, 12(sp)",
        "lw x6, 16(sp)",
        "lw x7, 20(sp)",
        "lw x8, 24(sp)",
        "lw x9, 28(sp)",
        "lw x10, 32(sp)",
        "lw x11, 36(sp)",
        "lw x12, 40(sp)",
        "lw x13, 44(sp)",
        "lw x14, 48(sp)",
        "lw x15, 52(sp)",
        "lw x16, 56(sp)",
        "lw x17, 60(sp)",
        "lw x18, 64(sp)",
        "lw x19, 68(sp)",
        "lw x20, 72(sp)",
        "lw x21, 76(sp)",
        "lw x22, 80(sp)",
        "lw x23, 84(sp)",
        "lw x24, 88(sp)",
        "lw x25, 92(sp)",
        "lw x26, 96(sp)",
        "lw x27, 100(sp)",
        "lw x28, 104(sp)",
        "lw x29, 108(sp)",
        "lw x30, 112(sp)",
        "lw x31, 116(sp)",

        "addi sp, sp, 128", // release stack space
        "mret",
    );
}