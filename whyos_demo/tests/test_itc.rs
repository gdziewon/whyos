#![no_std]
#![no_main]

use whyos_demo::{harness, check, TestResult};
use whyos::{Queue, Semaphore, Mutex};
use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};

static MUTEX_PRIO: Mutex<u32> = Mutex::new(0);
static EXEC_ORDER: AtomicU32 = AtomicU32::new(0);


fn test_mutex_priority() -> TestResult {
    EXEC_ORDER.store(0, Ordering::SeqCst);

    whyos::TaskBuilder::new(holder).priority(10).spawn().unwrap();
    whyos::sleep(10);

    whyos::TaskBuilder::new(waiter_med).priority(5).spawn().unwrap();
    whyos::TaskBuilder::new(waiter_high).priority(4).spawn().unwrap();
    whyos::TaskBuilder::new(waiter_low).priority(6).spawn().unwrap();

    whyos::sleep(200);

    // 1 (h) -> 2 (m) -> 3 (l) -> 123
    check!(*(MUTEX_PRIO.lock()) == 123, "tasks went in wrong order");

    Ok(())
}

extern "C" fn holder() {
    let _g = MUTEX_PRIO.lock();
    whyos::sleep(50);
}

extern "C" fn waiter_high() {
    let mut g = MUTEX_PRIO.lock();
    *g = *g * 10 + 1;
}

extern "C" fn waiter_med() {
    let mut g = MUTEX_PRIO.lock();
    *g = *g * 10 + 2;
}

extern "C" fn waiter_low() {
    let mut g = MUTEX_PRIO.lock();
    *g = *g * 10 + 3;
}


static SEM_COUNT: Semaphore = Semaphore::new(0, 3);
static CONS_FINISHED: AtomicBool = AtomicBool::new(false);

fn test_semaphore_blocking() -> TestResult {
    whyos::spawn_with_priority(consumer, 5).unwrap();
    whyos::sleep(20);

    check!(!CONS_FINISHED.load(Ordering::Relaxed));

    SEM_COUNT.signal();
    SEM_COUNT.signal();

    whyos::sleep(20);
    check!(CONS_FINISHED.load(Ordering::Relaxed));

    Ok(())
}

extern "C" fn consumer() {
    SEM_COUNT.wait();
    SEM_COUNT.wait();

    CONS_FINISHED.store(true, Ordering::Relaxed);
}

static QUEUE_SMALL: Queue<u8, 2> = Queue::new();

fn test_queue_try_send() -> TestResult {
    QUEUE_SMALL.send(10);
    QUEUE_SMALL.send(20);

    let res = QUEUE_SMALL.try_send(30);
    check!(res.is_err());
    check!(res.unwrap_err() == 30);

    let v = QUEUE_SMALL.receive();
    check!(v == 10);

    let res2 = QUEUE_SMALL.try_send(40);
    check!(res2.is_ok());

    Ok(())
}

static QUEUE_WRAP: Queue<u32, 3> = Queue::new();

fn test_queue_wrap() -> TestResult {
    QUEUE_WRAP.send(0);
    QUEUE_WRAP.send(1);
    QUEUE_WRAP.send(2);

    check!(QUEUE_WRAP.receive() == 0);
    check!(QUEUE_WRAP.receive() == 1);
    check!(QUEUE_WRAP.receive() == 2);

    QUEUE_WRAP.send(3);
    QUEUE_WRAP.send(4);
    QUEUE_WRAP.send(5);

    check!(QUEUE_WRAP.receive() == 3);
    check!(QUEUE_WRAP.receive() == 4);
    check!(QUEUE_WRAP.receive() == 5);

    Ok(())
}

static MUTEX_STRESS: Mutex<u32> = Mutex::new(0);

fn test_mutex_stress() -> TestResult {
    for _ in 0..4 {
        whyos::TaskBuilder::new(t5_worker).priority(5).spawn().unwrap();
    }

    whyos::sleep(500);

    // 4 tasks * 100 incr = 400
    check!(*(MUTEX_STRESS.lock()) == 400);

    Ok(())
}

extern "C" fn t5_worker() {
    for _ in 0..100 {
        let mut g = MUTEX_STRESS.lock();
        *g += 1;
    }
}


harness! {
    test_mutex_priority,
    test_semaphore_blocking,
    test_queue_try_send,
    test_queue_wrap,
    test_mutex_stress,
}