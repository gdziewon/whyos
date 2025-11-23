use core::{arch::naked_asm, cell::UnsafeCell, ptr, mem};
use cortex_m::peripheral::SCB;
use cortex_m_rt::exception;

const EXC_RETURN_THREAD_PSP: u32 = 0xFFFFFFFD;
const XPSR_THUMB: u32 = 0x01000000;

#[repr(C)]
#[derive(Debug)]
struct InitStackFrame {
    sw_frame: SwStackFrame,
    hw_frame: HwStackFrame
}

impl InitStackFrame {
    pub fn new(entry_point: extern "C" fn() -> !) -> Self {
        Self {
            sw_frame: SwStackFrame::new(),
            hw_frame: HwStackFrame::new(entry_point)
        }
    }
}

#[repr(C)]
#[derive(Debug)]
struct HwStackFrame { // popped automatically on interrupt return
    r0: u32,
    r1: u32,
    r2: u32,
    r3: u32,
    r12: u32,
    lr: u32,
    pc: u32,
    xpsr: u32,
}

impl HwStackFrame {
    pub fn new(entry_point: extern "C" fn() -> !) -> Self {
        Self {
            r0: 0, // todo: pass arg to task
            r1: 0x11111111,
            r2: 0x22222222,
            r3: 0x33333333,
            r12: 0xCCCCCCCC,
            lr: EXC_RETURN_THREAD_PSP, // todo: cleanup after task finishes?
            pc: entry_point as usize as u32,
            xpsr: XPSR_THUMB,
        }
    }
}

#[repr(C)]
#[derive(Debug)]
struct SwStackFrame { // popped manually in PendSV
    r4: u32,
    r5: u32,
    r6: u32,
    r7: u32,
    r8: u32,
    r9: u32,
    r10: u32,
    r11: u32,
}

impl SwStackFrame {
    pub fn new() -> Self {
        Self {
            r4: 0x44444444,
            r5: 0x55555555,
            r6: 0x66666666,
            r7: 0x77777777,
            r8: 0x88888888,
            r9: 0x99999999,
            r10: 0xAAAAAAAA,
            r11: 0xBBBBBBBB,
        }
    }
}

#[repr(C)]
pub struct Tcb {
    pub sp: u32
}

#[unsafe(no_mangle)]
pub static mut TASKS: [Tcb; 2] = [Tcb { sp: 0 }, Tcb { sp: 0 }];
static mut CURRENT_TASK: usize = 0;

#[repr(C, align(8))]
pub struct Stack<const SIZE: usize> {
    pub data: UnsafeCell<[u8; SIZE]>,
}

unsafe impl<const S: usize> Sync for Stack<S> {}

impl<const SIZE: usize> Stack<SIZE> {
    pub const fn new() -> Self {
        Self { data: UnsafeCell::new([0; SIZE]) }
    }

    pub fn init(&self, entry_point: extern "C" fn() -> !) -> u32 {
        let stack_ptr = self.data.get() as *mut u8;
        let stack_len = SIZE;

        let stack_top = unsafe { stack_ptr.add(stack_len) };
        //let aligned_top = stack_top as usize & !7; // shouldn't be needed, stack is 8byte aligned anyway

        let init_frame = InitStackFrame::new(entry_point);
        let frame_ptr = (stack_top as usize - mem::size_of::<InitStackFrame>()) as *mut InitStackFrame;

        unsafe { ptr::write(frame_ptr, init_frame) };
        frame_ptr as u32
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn save_current_sp(sp: u32) {
    unsafe {
        TASKS[CURRENT_TASK].sp = sp;
    }
}

#[unsafe(no_mangle)]
unsafe extern "C" fn load_next_sp() -> u32 {
    unsafe {
        CURRENT_TASK = (CURRENT_TASK + 1) % 2; // todo: impl priorities
        TASKS[CURRENT_TASK].sp
    }
}

#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn PendSV() {
    naked_asm!(
        "mrs r0, psp",           // move psp to r0
        "isb",                   // sync barrier
        "stmdb r0!, {{r4-r11}}", // push regs r4-r11 onto r0 (psp) and update it
        "push {{lr}}",           // push LR (tells the cpu what it was doing before it was interrupted)

        "bl save_current_sp",    // save stack pointer (psp)
        "bl load_next_sp",       // switch task, get new stack pointer (into r0)

        "pop {{r1}}",            // pop LR value
        "mov lr, r1",            // move it to LR reg
        "ldmia r0!, {{r4-r11}}", // pop saved regs of new task, update r0
        "msr psp, r0",           // set psp to r0
        "isb",                   // sync barrier
        "bx lr",                 // pop hw frame and run the task (thread mode, psp)
    );
}

#[unsafe(no_mangle)]
#[unsafe(naked)]
pub unsafe extern "C" fn SVCall() {
    naked_asm!(
        "ldr r0, =TASKS",       // load address of TASKS array
        "ldr r0, [r0]",         // load first task sp (sp needs to be first field)
        "ldmia r0!, {{r4-r11}}",// discard software frame (update r0 to point at hardware frame)

        "msr psp, r0",          // set psp to r0

        "mov r0, {EXC_VAL}",
        "mov lr, r0",           // set lr to EXC_RETURN_THREAD_PSP

        "bx lr",                // pop hw frame and run the task (thread mode, psp)
        EXC_VAL = const EXC_RETURN_THREAD_PSP,
    );
}

#[exception]
fn SysTick() {
    SCB::set_pendsv(); // handle switch in PendSV
}