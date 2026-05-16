#![no_std]
#![no_main]

use whyos::TaskHandle;
use whyos_demo::{harness, check, TestResult};
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

static COUNTER: AtomicU32 = AtomicU32::new(0);
static KILL_COUNTER: AtomicU32 = AtomicU32::new(0);

static TEST_MUTEX: whyos::Mutex<u32> = whyos::Mutex::new(0);
static LOW_RAN: AtomicU32 = AtomicU32::new(0);

fn test_suspend_resume() -> TestResult {
    COUNTER.store(0, Ordering::Relaxed);

    let handle = whyos::spawn_with_priority(worker_count, 10).unwrap();

    whyos::sleep(50);
    let val_before = COUNTER.load(Ordering::Relaxed);
    check!(val_before > 0, "Worker didnt start");

    handle.suspend().unwrap();
    let info = handle.info().unwrap();
    check!(matches!(info.state, whyos::TaskState::Suspended(_)), "Task state is not Suspended, got {:?}", info.state);

    whyos::sleep(100);

    let val_suspended = COUNTER.load(Ordering::Relaxed);

    check!(val_suspended == val_before, "Worker ran while suspended: before={}, suspended={}", val_before, val_suspended);

    info.handle.resume().unwrap();
    whyos::sleep(50);

    let val_after = COUNTER.load(Ordering::Relaxed);

    check!(val_after > val_suspended, "Worker didnt resume: suspended={}, after={}", val_suspended, val_after);

    info.handle.kill().unwrap();
    Ok(())
}

extern "C" fn worker_count() {
    loop {
        COUNTER.fetch_add(1, Ordering::Relaxed);
        whyos::sleep(10);
    }
}

// Test memory reclamation
fn test_reincarnation() -> TestResult {
    let initial_tasks = whyos::allocated().count();

    for _ in 0..100 {
        whyos::spawn_with_priority(worker_die, 10)
            .map_err(|_| "Failed to reap zombies")?;
        whyos::sleep(1);
    }
    whyos::sleep(2);

    let current_tasks = whyos::allocated().count();
        check!(
            current_tasks <= initial_tasks,
            "Zombies were not reaped! Expected less then {} tasks, got {}",
            initial_tasks,
            current_tasks
        );

    Ok(())
}

extern "C" fn worker_die() {
    whyos::exit()
}

// Tests how Mutex will handle suspended, waiting for lock task with high prio
fn test_suspend_mutex_inversion() -> TestResult {
    LOW_RAN.store(0, Ordering::Relaxed);

    whyos::spawn_with_priority(mutex_holder, 6).unwrap();
    whyos::sleep(5);

    whyos::spawn_with_priority(mutex_waiter_high, 4).unwrap();
    let high_h = whyos::spawn_with_priority(mutex_waiter_high, 4).unwrap();
    whyos::sleep(5);

    let _low_h = whyos::TaskBuilder::with_value(mutex_waiter_low, high_h.as_u32()).priority(5).spawn().unwrap();
    whyos::sleep(5);

    high_h.suspend().unwrap();

    let info = high_h.info().unwrap();
    check!(matches!(info.state, whyos::TaskState::Suspended(_)), "Task state is not Suspended, got {:?}", info.state);

    whyos::sleep(100);

    let low_val = LOW_RAN.load(Ordering::Relaxed);

    whyos::sleep(50);

    check!(low_val == 1, "Low priority task was starved - high priority task ran instead (probably). low_val={}", low_val);

    Ok(())
}

extern "C" fn mutex_holder() {
    let _g = TEST_MUTEX.lock();
    whyos::sleep(50);
}

extern "C" fn mutex_waiter_high() {
    let _g = TEST_MUTEX.lock();
}

extern "C" fn mutex_waiter_low(high_tid: u32) {
    let _g = TEST_MUTEX.lock();

    LOW_RAN.store(1, Ordering::Relaxed);

    let h = TaskHandle::from_u32(high_tid).unwrap();
    h.resume().unwrap();
}

static STOP_FEEDING: AtomicBool = AtomicBool::new(false);

fn test_watchdog_feeding() -> TestResult {
    let _feeder = whyos::spawn_with_priority(watchdog_feeder, 3).unwrap();
    whyos::sleep(10);
    STOP_FEEDING.store(true, Ordering::Relaxed);

    Ok(())
}

extern "C" fn watchdog_feeder() {
    whyos::wdt_sub(2);
    while !STOP_FEEDING.load(Ordering::Relaxed) {
        whyos::wdt_feed();
    }
    whyos::wdt_unsub();
}

fn test_watchdog_starving() -> TestResult {
    let h = whyos::spawn_with_priority(watchdog_starver, 3).unwrap();
    whyos::sleep(10);

    check!(h.info().is_err(), "Task didn't die");

    Ok(())
}

extern "C" fn watchdog_starver() {
    whyos::wdt_sub(1);
    loop {}
}

static SELF_SUSPEND_FLAG: AtomicBool = AtomicBool::new(false);

fn test_self_suspend() -> TestResult {
    let h = whyos::spawn(self_suspender).unwrap();

    whyos::sleep(10);
    check!(SELF_SUSPEND_FLAG.load(Ordering::Relaxed), "Task didn't run");

    let info = h.info().unwrap();
    check!(matches!(info.state, whyos::TaskState::Suspended(_)), "Task didn't suspend itself");

    info.handle.resume().unwrap();

    whyos::sleep(10);
    check!(!SELF_SUSPEND_FLAG.load(Ordering::Relaxed), "Task didn't resume");

    Ok(())
}

extern "C" fn self_suspender() {
    SELF_SUSPEND_FLAG.store(true, Ordering::Relaxed);
    whyos::my_handle().suspend().unwrap();
    SELF_SUSPEND_FLAG.store(false, Ordering::Relaxed);
}

fn test_kill_task() -> TestResult {
    KILL_COUNTER.store(0, Ordering::Relaxed);

    let h = whyos::spawn_with_priority(kill_worker, 10).unwrap();

    whyos::sleep(10);
    let before_kill = KILL_COUNTER.load(Ordering::Relaxed);
    check!(before_kill > 0, "Kill worker did not start");

    h.kill().unwrap();

    let info = h.info().unwrap();
    check!(matches!(info.state, whyos::TaskState::Zombie), "Task state is not Zombie after kill, got {:?}", info.state);

    whyos::sleep(20);
    let after_kill = KILL_COUNTER.load(Ordering::Relaxed);
    check!(after_kill == before_kill, "Killed task kept running: before={}, after={}", before_kill, after_kill);

    whyos::reclaim_memory();
    check!(info.handle.info().is_err(), "Killed task was not reclaimed");

    Ok(())
}

extern "C" fn kill_worker() {
    loop {
        KILL_COUNTER.fetch_add(1, Ordering::Relaxed);
        whyos::sleep(5);
    }
}

harness! {
    test_suspend_resume,
    test_reincarnation,
    test_suspend_mutex_inversion,
    test_watchdog_feeding,
    test_watchdog_starving,
    test_self_suspend,
    test_kill_task,
}