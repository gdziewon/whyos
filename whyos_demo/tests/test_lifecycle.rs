#![no_std]
#![no_main]

use whyos_demo::{harness, check, TestResult};
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

static COUNTER: AtomicU32 = AtomicU32::new(0);
static STOP_FLAG: AtomicBool = AtomicBool::new(false);

static TEST_MUTEX: whyos::Mutex<u32> = whyos::Mutex::new(0);
static LOW_RAN: AtomicU32 = AtomicU32::new(0);

fn test_suspend_resume() -> TestResult {
    COUNTER.store(0, Ordering::Relaxed);

    let tid = whyos::spawn_with_priority(worker_count, 10).unwrap();

    whyos::sleep(50);
    let val_before = COUNTER.load(Ordering::Relaxed);
    check!(val_before > 0, "Worker didnt start");

    whyos::suspend(tid).unwrap();
    let info = whyos::task_info(tid).unwrap();
    check!(matches!(info.state, whyos::TaskState::Suspended(_)));

    whyos::sleep(100);

    let val_suspended = COUNTER.load(Ordering::Relaxed);

    check!(val_suspended == val_before, "Worker ran while suspended");

    whyos::resume(tid).unwrap();
    whyos::sleep(50);

    let val_after = COUNTER.load(Ordering::Relaxed);

    check!(val_after > val_suspended, "Worker didnt resume");

    STOP_FLAG.store(true, Ordering::Relaxed);
    Ok(())
}

extern "C" fn worker_count() {
    while !STOP_FLAG.load(Ordering::Relaxed) {
        COUNTER.fetch_add(1, Ordering::Relaxed);
        whyos::sleep(10);
    }
}

// Test memory reclamation
fn test_reincarnation() -> TestResult {
    for _ in 0..100 {
        whyos::spawn_with_priority(worker_die, 10)
            .expect("Failed to reap zombies");
        whyos::sleep(2);
    }

    Ok(())
}

extern "C" fn worker_die() {
    whyos::exit();
}

// Tests how Mutex will handle suspended, waiting for lock task with high prio
fn test_suspend_mutex_inversion() -> TestResult {
    LOW_RAN.store(0, Ordering::Relaxed);

    whyos::spawn_with_priority(mutex_holder, 6).unwrap();
    whyos::sleep(5);

    whyos::spawn_with_priority(mutex_waiter_high, 4).unwrap();
    let high_tid = whyos::spawn_with_priority(mutex_waiter_high, 4).unwrap();
    whyos::sleep(5);

    let _low_tid = whyos::TaskBuilder::with_value(mutex_waiter_low, high_tid).priority(5).spawn().unwrap();
    whyos::sleep(5);

    whyos::suspend(high_tid).unwrap();

    let info = whyos::task_info(high_tid).unwrap();
    check!(matches!(info.state, whyos::TaskState::Suspended(_)));

    whyos::sleep(100);

    let low_val = LOW_RAN.load(Ordering::Relaxed);

    whyos::sleep(50);

    check!(low_val == 1, "Low priority task was starved - high priority task ran instead (probably)");

    Ok(())
}

extern "C" fn mutex_holder() {
    let _g = TEST_MUTEX.lock();
    whyos::sleep(50);
}

extern "C" fn mutex_waiter_high() {
    let _g = TEST_MUTEX.lock();
}

extern "C" fn mutex_waiter_low(high_tid: whyos::TaskId) {
    let _g = TEST_MUTEX.lock();

    LOW_RAN.store(1, Ordering::Relaxed);

    whyos::resume(high_tid).unwrap();
}

harness! {
    test_suspend_resume,
    test_reincarnation,
    test_suspend_mutex_inversion
}