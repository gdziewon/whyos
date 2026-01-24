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
            // yield if we woke someone or item was not sent
            let should_yield = critical_section::with(|cs| {
                let mut state = self.state.borrow(cs).borrow_mut();

                let curr_tid = scheduler::get_current_tid();
                if state.count == CAPACITY { // queue is full, cant send
                    scheduler::block_current_task();
                    state.prod_waiting.add(curr_tid);
                    true

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

                    state.prod_waiting.remove(curr_tid); // needed for weird stuff with suspend/resume FIXME

                    if let Some(tid) = pop_highest_prio(&mut state.cons_waiting) {
                        scheduler::wake_task(tid);
                        true
                    } else {
                        false
                    }
                }
            });

            if item_slot.is_none() { // we sent the item
                if should_yield {
                    scheduler::yield_now();
                }
                return;
            }

            // blocked
            scheduler::yield_now();
        }
    }

    #[inline]
    pub fn try_send(&self, item: T) -> Result<(), T> { // we are returning the item in Err(T) if try_send failed
        critical_section::with(|cs| {
            let mut state = self.state.borrow(cs).borrow_mut();

            if state.count == CAPACITY {
                return Err(item); // full queue
            }

            let maybe_ptr = self.data.get();
            unsafe {
                let data_ptr = (*maybe_ptr).as_mut_ptr() as *mut T;
                data_ptr.add(state.write_idx).write(item);
            }

            state.write_idx = (state.write_idx + 1) % CAPACITY;
            state.count += 1;

            if let Some(tid) = pop_highest_prio(&mut state.cons_waiting) {
                scheduler::wake_task(tid);
                // wont yield here, caller should do it manually if needed
            }

            Ok(())
        })
    }

    pub fn receive(&self) -> T {
        let mut received_data: Option<T> = None;

        loop {
            // yield if we woke someone
            let should_yield = critical_section::with(|cs| {
                let mut state = self.state.borrow(cs).borrow_mut();

                let curr_tid = scheduler::get_current_tid();
                if state.count == 0 { // no data to receive = block
                    scheduler::block_current_task();
                    state.cons_waiting.add(curr_tid);
                    false

                } else { // get data
                    let maybe_ptr = self.data.get();

                    received_data = unsafe {
                        let data_start_ptr = (*maybe_ptr).as_ptr() as *const T;
                        let val = data_start_ptr.add(state.read_idx).read();
                        Some(val)
                    };

                    state.read_idx = (state.read_idx + 1) % CAPACITY;
                    state.count -= 1;

                    state.cons_waiting.remove(curr_tid); // needed for weird stuff with suspend/resume FIXME

                    if let Some(tid) = pop_highest_prio(&mut state.prod_waiting) {
                        scheduler::wake_task(tid);
                        true
                    } else {
                        false
                    }
                }
            });

            if let Some(data) = received_data {
                if should_yield {
                    scheduler::yield_now();
                }
                return data;
            }

            scheduler::yield_now();
        }
    }

    #[inline]
    pub fn try_receive(&self) -> Option<T> {
        critical_section::with(|cs| {
            let mut state = self.state.borrow(cs).borrow_mut();

            if state.count == 0 {
                return None; // queue empty
            }

            let maybe_ptr = self.data.get();
            let val = unsafe {
                let data_ptr = (*maybe_ptr).as_ptr() as *const T;
                data_ptr.add(state.read_idx).read()
            };

            state.read_idx = (state.read_idx + 1) % CAPACITY;
            state.count -= 1;

            if let Some(tid) = pop_highest_prio(&mut state.prod_waiting) {
                scheduler::wake_task(tid);
                // wont yield here, caller should do it manually if needed
            }

            Some(val)
        })
    }

    #[inline]
    pub fn len(&self) -> usize {
        critical_section::with(|cs| {
            self.state.borrow(cs).borrow().count
        })
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn is_full(&self) -> bool {
        self.len() == CAPACITY
    }
}