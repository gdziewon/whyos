use core::{cell::{RefCell, UnsafeCell}, mem::MaybeUninit};
use critical_section::Mutex as CSMutex;

use crate::{task::TaskMap, scheduler, itc::pop_highest_prio};

pub struct Queue<T, const N: usize> {
    data: UnsafeCell<MaybeUninit<[T; N]>>,
    state: CSMutex<RefCell<QueueState>>
}

struct QueueState {
    count: usize,
    write_idx: usize,
    read_idx: usize,
    prod_waiting: TaskMap,
    cons_waiting: TaskMap
}

impl QueueState {
    const fn new() -> Self {
        Self {
            count: 0,
            write_idx: 0,
            read_idx: 0,
            prod_waiting: TaskMap::new(),
            cons_waiting: TaskMap::new()
        }
    }
}

unsafe impl<T: Send, const N: usize> Sync for Queue<T, N> {}

impl<T: Send, const CAPACITY: usize> Queue<T, CAPACITY> {
    pub const fn new() -> Self {
        Self {
            data: UnsafeCell::new(MaybeUninit::uninit()),
            state: CSMutex::new(RefCell::new(QueueState::new()))
        }
    }

    pub fn send(&self, item: T) {
        let mut item_slot = Some(item);

        loop {
            let mut woken = false;

            critical_section::with(|cs| {
                let mut state = self.state.borrow(cs).borrow_mut();

                if state.count == CAPACITY { // queue is full, cant send
                    scheduler::block_current_task();
                    let curr_tid = scheduler::get_current_tid();
                    state.prod_waiting.add(curr_tid);

                } else {
                    let val = item_slot.take().unwrap(); // unwrap is safe bcs we know we haven't sent it yet

                    let maybe_ptr = self.data.get();

                    // this is safe, we know the memory lives and is valid
                    unsafe {
                        let data_start_ptr = (*maybe_ptr).as_mut_ptr() as *mut T;
                        data_start_ptr.add(state.write_idx).write(val);
                    }

                    state.write_idx = (state.write_idx + 1) % CAPACITY;
                    state.count += 1;

                    if let Some(tid) = pop_highest_prio(&mut state.cons_waiting) {
                        scheduler::wake_task(tid);
                        woken = true;
                    }
                }
            });

            if item_slot.is_none() { // we sent the item
                if woken {
                    scheduler::yield_now();
                }
                return;
            } else {
                scheduler::yield_now();
            }
        }
    }

    pub fn receive(&self) -> T {
        let mut received_data: Option<T> = None;

        loop {
            let mut woken = false;

            critical_section::with(|cs| {
                let mut state = self.state.borrow(cs).borrow_mut();

                if state.count == 0 { // no data to receive = block
                    scheduler::block_current_task();
                    let curr_tid = scheduler::get_current_tid();
                    state.cons_waiting.add(curr_tid);

                } else { // get data

                    let maybe_ptr = self.data.get();

                    unsafe {
                        let data_start_ptr = (*maybe_ptr).as_ptr() as *const T;
                        let val = data_start_ptr.add(state.read_idx).read();
                        received_data = Some(val);
                    }


                    state.read_idx = (state.read_idx + 1) % CAPACITY;
                    state.count -= 1;

                    if let Some(tid) = pop_highest_prio(&mut state.prod_waiting) {
                        scheduler::wake_task(tid);
                        woken = true;
                    }
                }
            });

            if let Some(data) = received_data {
                if woken {
                    scheduler::yield_now();
                }
                return data;
            } else {
                scheduler::yield_now();
            }
        }

    }
}