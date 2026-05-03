

// https://www2.eecs.berkeley.edu/Pubs/TechRpts/2016/EECS-2016-161.pdf#page=32
// enable machine interrupts and go into M-mode
const MPIE_BIT: u32 = 7;
const MPP_DOUBLE_BIT: u32 = 11;
const MSTATUS_TASK_START: u32 = (1 << MPIE_BIT) | (0b11 << MPP_DOUBLE_BIT);

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct InitStackFrame { // skipping zero reg and sp
    pub ra: u32,
    pub gp: u32, // i know it doesnt look very good, but I like how transparent it is
    pub tp: u32,
    pub t0: u32,
    pub t1: u32,
    pub t2: u32,
    pub s0: u32,
    pub s1: u32,
    pub a0: u32,
    pub a1: u32,
    pub a2: u32,
    pub a3: u32,
    pub a4: u32,
    pub a5: u32,
    pub a6: u32,
    pub a7: u32,
    pub s2: u32,
    pub s3: u32,
    pub s4: u32,
    pub s5: u32,
    pub s6: u32,
    pub s7: u32,
    pub s8: u32,
    pub s9: u32,
    pub s10: u32,
    pub s11: u32,
    pub t3: u32,
    pub t4: u32,
    pub t5: u32,
    pub t6: u32,

    // CSRs
    pub mepc: u32,
    pub mstatus: u32,
}

impl InitStackFrame {
    pub fn new(entry_point: usize, arg: usize, return_handler: usize) -> Self {
        // gp and tp are the same for the whole program, we need to pass them to new tasks
        let (gp, tp): (u32, u32);
        unsafe {
            core::arch::asm!("mv {0}, gp", out(reg) gp);
            core::arch::asm!("mv {0}, tp", out(reg) tp);
        }

        Self {
            ra: return_handler as u32,
            gp,
            tp,
            t0: 0, t1: 0, t2: 0,
            s0: 0, s1: 0,
            a0: arg as u32,
            a1: 0, a2: 0, a3: 0, a4: 0, a5: 0, a6: 0, a7: 0,
            s2: 0, s3: 0, s4: 0, s5: 0, s6: 0, s7: 0, s8: 0, s9: 0, s10: 0, s11: 0,
            t3: 0, t4: 0, t5: 0, t6: 0,

            mepc: entry_point as u32,
            mstatus: MSTATUS_TASK_START,
        }
    }
}