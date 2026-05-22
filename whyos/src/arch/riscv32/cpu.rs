use riscv::register::mstatus;

#[inline(always)]
pub fn bkpt() { unsafe { riscv::asm::ebreak() } }

#[inline(always)]
pub fn wfi() { riscv::asm::wfi() }

#[inline(always)]
pub fn yield_now() { unsafe { riscv::asm::ecall() } }

#[inline(always)]
pub fn reset() -> ! {
    // riscv standard doesnt define reset
    panic!("WhyOS: RISC-V doesn't support reset")
}

// on interrupt, riscv sets MIE to 0, so if mie=1 then we're in task
#[inline(always)]
pub fn is_in_task() -> bool {mstatus::read().mie() }


#[inline(always)]
pub fn cycle_count() -> u32 {
    riscv::register::mcycle::read() as u32
}