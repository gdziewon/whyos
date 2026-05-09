#![no_std]
#![no_main]

use whyos_demo::{check, harness, TestResult};
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

static COUNTER_A: AtomicU32 = AtomicU32::new(0);
static COUNTER_B: AtomicU32 = AtomicU32::new(0);
static COUNTER_C: AtomicU32 = AtomicU32::new(0);
static COUNTER_D: AtomicU32 = AtomicU32::new(0);

static ACTIVE_COUNT: AtomicU32 = AtomicU32::new(0);

static STOP_FLAG: AtomicBool = AtomicBool::new(false);

// Tests 30 tasks with the same priority running at the same time
fn test_saturation() -> TestResult {
    ACTIVE_COUNT.store(0, Ordering::Relaxed);

    let mut spawned = 0;
    for _ in 0..30 {
        let res = whyos::spawn_with_priority(tiny_task, 10);

        if res.is_ok() {
            spawned += 1;
        } else {
            break;
        }
    }

    whyos::sleep(50);

    let active = ACTIVE_COUNT.load(Ordering::Relaxed);

    check!(active == spawned as u32, "Not all spawned tasks ran");

    Ok(())
}

extern "C" fn tiny_task() {
    ACTIVE_COUNT.fetch_add(1, Ordering::Relaxed);
}

// Tests strict preemption, low prio task should never run in this case
fn test_starvation() -> TestResult {
    COUNTER_A.store(0, Ordering::Relaxed);
    COUNTER_B.store(0, Ordering::Relaxed);

    let low_h = whyos::spawn_with_priority(worker_low, 20).unwrap();

    let high_h = whyos::spawn_with_priority(worker_high_hog, 5).unwrap();

    whyos::sleep(200);

    let low_cnt = COUNTER_A.load(Ordering::Relaxed);
    let high_cnt = COUNTER_B.load(Ordering::Relaxed);

    defmt::info!("high_cnt {}", high_cnt);
    check!(high_cnt > 3730000, "High priority task didn't run enough"); // it's also for me, to know if performence is degrading
    check!(low_cnt == 0, "Low priority task ran! Scheduler failed strict preemption");

    low_h.kill().unwrap();
    high_h.kill().unwrap();

    Ok(())
}

extern "C" fn worker_low() {
    COUNTER_A.fetch_add(1, Ordering::Relaxed);
}

extern "C" fn worker_high_hog() {
    loop {
        COUNTER_B.fetch_add(1, Ordering::Relaxed);
    }
}

// Test fairness for same prio tasks
fn test_fairness() -> TestResult {
    STOP_FLAG.store(false, Ordering::Relaxed);
    COUNTER_A.store(0, Ordering::Relaxed);
    COUNTER_B.store(0, Ordering::Relaxed);
    COUNTER_C.store(0, Ordering::Relaxed);
    COUNTER_D.store(0, Ordering::Relaxed);

    let r1 = whyos::TaskBuilder::with_static_ref(rr_worker, &COUNTER_A).priority(10).spawn().unwrap();
    let r2 = whyos::TaskBuilder::with_static_ref(rr_worker, &COUNTER_B).priority(10).spawn().unwrap();
    let r3 =whyos::TaskBuilder::with_static_ref(rr_worker, &COUNTER_C).priority(10).spawn().unwrap();
    let r4 = whyos::TaskBuilder::with_static_ref(rr_worker, &COUNTER_D).priority(10).spawn().unwrap();

    whyos::sleep(100);
    STOP_FLAG.store(true, Ordering::Relaxed);
    let a = COUNTER_A.load(Ordering::Relaxed);
    let b = COUNTER_B.load(Ordering::Relaxed);
    let c = COUNTER_C.load(Ordering::Relaxed);
    let d = COUNTER_D.load(Ordering::Relaxed);

    let min = a.min(b).min(c).min(d);
    let max = a.max(b).max(c).max(d);

    defmt::info!("a: {}, b: {}, c: {}, d: {}", a, b, c, d);
    check!(min > 0, "One or more tasks starved");
    check!(max < min * 2, "Unfair scheduling");

    r1.kill().unwrap();
    r2.kill().unwrap();
    r3.kill().unwrap();
    r4.kill().unwrap();

    Ok(())
}

extern "C" fn rr_worker(flag: &'static AtomicU32) { loop { flag.fetch_add(1, Ordering::Relaxed); whyos::yield_cpu(); } }


static COUNTER_PING: AtomicU32 = AtomicU32::new(0);
static COUNTER_PONG: AtomicU32 = AtomicU32::new(0);

extern "C" fn ping() {
    loop {
        COUNTER_PING.fetch_add(1, Ordering::Relaxed);
        whyos::yield_cpu();
    }
}

extern "C" fn pong() {
    loop {
        COUNTER_PONG.fetch_add(1, Ordering::Relaxed);
        whyos::yield_cpu();
    }
}

fn test_pingpong() -> TestResult {
    let ping = whyos::spawn_with_priority(ping, 7).unwrap();
    let pong = whyos::spawn_with_priority(pong, 7).unwrap();

    whyos::sleep(100);
    let sum = COUNTER_PING.load(Ordering::Relaxed) + COUNTER_PONG.load(Ordering::Relaxed);
    defmt::info!("Ping pong sum: {}", sum);
    check!(sum > 62000, "Lost some performence");

    ping.kill().unwrap();
    pong.kill().unwrap();

    Ok(())
}

harness! {
    test_saturation,
    test_starvation,
    test_fairness,
    test_pingpong
}