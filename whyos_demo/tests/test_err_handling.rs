#![no_std]
#![no_main]

use whyos_demo::{check, harness, TestResult};

fn test_oom() -> TestResult {
    let res = whyos::TaskBuilder::new(dummy_task)
        .stack_size(whyos::StackSize::bytes(1024 * 1024))
        .spawn();

    check!(res.is_err(), "Spawned task with too large stack");
    check!(res.unwrap_err() == whyos::WhyError::OutOfMemory);

    Ok(())
}

fn test_max_tasks() -> TestResult {
    let mut spawned = 0;
    for _ in 0..40 {
        if whyos::spawn_with_priority(dummy_task, 10).is_ok() {
            spawned += 1;
        }
    }

    check!(spawned > 0 && spawned <= whyos::MAX_TASKS, "Spawned too many tasks");

    Ok(())
}

extern "C" fn dummy_task() {
    whyos::sleep(5);
}

harness! {
    test_oom,
    test_max_tasks
}