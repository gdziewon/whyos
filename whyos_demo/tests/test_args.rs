#![no_std]
#![no_main]

use whyos_demo::{check, harness, TestResult};

static TASK_ARG: whyos::Mutex<u32> = whyos::Mutex::new(0);

fn test_value() -> TestResult {
    let arg: u32 = 65;

    whyos::TaskBuilder::with_value(takes_value, arg).spawn().unwrap();
    whyos::sleep(100);

    check!(*(TASK_ARG.lock()) == arg);
    Ok(())
}

extern "C" fn takes_value(a: u32) {
    *(TASK_ARG.lock()) = a;
}

fn test_ptr_mut() -> TestResult {
    static mut ARG: u32 = 42;
    let arg_ptr = &raw mut ARG;

    unsafe { whyos::TaskBuilder::with_ptr_mut(takes_ptr_mut, arg_ptr).spawn().unwrap() };
    whyos::sleep(100);

    check!(*(TASK_ARG.lock()) == unsafe { ARG });
    Ok(())
}

extern "C" fn takes_ptr_mut(a: *mut u32) {
    *(TASK_ARG.lock()) = unsafe { *a };
}

fn test_static_mut() -> TestResult {
    let buf_idx1 = 4;
    let buffer: &'static mut [u32; 3] = cortex_m::singleton!(: [u32; 3] = [1, buf_idx1, 3]).unwrap();
    whyos::TaskBuilder::with_static_mut(takes_static_mut, buffer).spawn().unwrap();
    whyos::sleep(100);

    check!(*(TASK_ARG.lock()) == buf_idx1);
    Ok(())
}

extern "C" fn takes_static_mut(a: &'static mut [u32; 3]) {
    *(TASK_ARG.lock()) = a[1];
}

harness!{
    test_value,
    test_static_mut,
    test_ptr_mut,
}