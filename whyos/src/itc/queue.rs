use core::{cell::{RefCell, UnsafeCell}, mem::MaybeUninit};
use critical_section::Mutex as CSMutex;

use crate::{scheduler, itc::WaitQueue};
use crate::utils::log;

/// A bounded, Multi-Producer / Multi-Consumer (MPMC) queue.
///
/// The queue is implemented as a ring buffer with compile-time `CAPACITY`.
/// Items stored in the queue are moved into the buffer.
pub struct Queue<T, const N: usize> {
    data: UnsafeCell<MaybeUninit<[T; N]>>,
    state: CSMutex<RefCell<QueueState>>
}

struct QueueState {
    count: usize,
    write_idx: usize,
    read_idx: usize,
    prod_waiting: WaitQueue,
    cons_waiting: WaitQueue
}

impl QueueState {
    const fn new() -> Self {
        Self {
            count: 0,
            write_idx: 0,
            read_idx: 0,
            prod_waiting: WaitQueue::new(),
            cons_waiting: WaitQueue::new()
        }
    }
}

unsafe impl<T: Send, const N: usize> Sync for Queue<T, N> {}

impl<T: Send, const CAPACITY: usize> Queue<T, CAPACITY> {
    /// Creates a new bounded queue with capacity `CAPACITY`.
    ///
    /// The queue requires `CAPACITY > 0`.
    pub const fn new() -> Self {
        Self {
            data: UnsafeCell::new(MaybeUninit::uninit()),
            state: CSMutex::new(RefCell::new(QueueState::new()))
        }
    }

    /// Enqueues an item, blocking the current task if the queue is full.
    pub fn send(&self, item: T) {
        let mut item_slot = Some(item);

        loop {
            // yield if we woke someone or item was not sent
            let should_yield = critical_section::with(|cs| {
                let mut state = self.state.borrow_ref_mut(cs);

                if state.count == CAPACITY { // queue is full, cant send
                    log::debug!("Queue full, producer blocking");
                    state.prod_waiting.block_current();
                    true

                } else {
                    log::trace!("Queue send success");
                    let val = item_slot.take().unwrap(); // unwrap is safe bcs we know we haven't sent it yet

                    let maybe_ptr = self.data.get();

                    // this is safe, we know the memory lives and is valid
                    unsafe {
                        let data_start_ptr = (*maybe_ptr).as_mut_ptr() as *mut T;
                        data_start_ptr.add(state.write_idx).write(val);
                    }

                    state.write_idx = (state.write_idx + 1) % CAPACITY;
                    state.count += 1;

                    state.prod_waiting.remove_current(); // needed for weird stuff with suspend/resume FIXME

                    state.cons_waiting.wake_highest_prio()
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

    /// Attempts to enqueue an item without blocking.
    ///
    /// Returns `Ok(())` on success or `Err(item)` if the queue is full.
    #[inline]
    pub fn try_send(&self, item: T) -> Result<(), T> { // we are returning the item in Err(T) if try_send failed
        critical_section::with(|cs| {
            let mut state = self.state.borrow_ref_mut(cs);

            if state.count == CAPACITY {
                log::trace!("Queue try_send failed - full");
                return Err(item); // full queue
            }

            let maybe_ptr = self.data.get();
            unsafe {
                let data_ptr = (*maybe_ptr).as_mut_ptr() as *mut T;
                data_ptr.add(state.write_idx).write(item);
            }

            state.write_idx = (state.write_idx + 1) % CAPACITY;
            state.count += 1;

            state.cons_waiting.wake_highest_prio(); // wont yield here, caller should do it manually if needed

            log::trace!("Queue try_send success, count {}", state.count);

            Ok(())
        })
    }

    /// Dequeues an item, blocking the current task if the queue is empty.
    pub fn receive(&self) -> T {
        let mut received_data: Option<T> = None;

        loop {
            // yield if we woke someone
            let should_yield = critical_section::with(|cs| {
                let mut state = self.state.borrow_ref_mut(cs);

                if state.count == 0 { // no data to receive = block
                    log::debug!("Queue empty, consumer blocking");
                    state.cons_waiting.block_current();
                    false

                } else { // get data
                    log::trace!("Queue receive success");
                    let maybe_ptr = self.data.get();

                    received_data = unsafe {
                        let data_start_ptr = (*maybe_ptr).as_ptr() as *const T;
                        let val = data_start_ptr.add(state.read_idx).read();
                        Some(val)
                    };

                    state.read_idx = (state.read_idx + 1) % CAPACITY;
                    state.count -= 1;

                    state.cons_waiting.remove_current(); // needed for weird stuff with suspend/resume FIXME

                    state.prod_waiting.wake_highest_prio()
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

    /// Attempts to dequeue an item without blocking.
    ///
    /// Returns `Some(item)` on success or `None` if the queue is empty.
    #[inline]
    pub fn try_receive(&self) -> Option<T> {
        critical_section::with(|cs| {
            let mut state = self.state.borrow_ref_mut(cs);

            if state.count == 0 {
                log::trace!("Queue try_receive - empty");
                return None; // queue empty
            }

            let maybe_ptr = self.data.get();
            let val = unsafe {
                let data_ptr = (*maybe_ptr).as_ptr() as *const T;
                data_ptr.add(state.read_idx).read()
            };

            state.read_idx = (state.read_idx + 1) % CAPACITY;
            state.count -= 1;

            state.prod_waiting.wake_highest_prio(); // wont yield here, caller should do it manually if needed

            log::trace!("Queue try_receive success, count {}", state.count);
            Some(val)
        })
    }

    /// Returns the number of items currently stored in the queue.
    #[inline]
    pub fn len(&self) -> usize {
        critical_section::with(|cs| {
            self.state.borrow(cs).borrow().count
        })
    }

    /// Returns `true` if the queue contains no items.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns `true` if the queue is full.
    #[inline]
    pub fn is_full(&self) -> bool {
        self.len() == CAPACITY
    }
}

impl<T: Send, const CAPACITY: usize> Default for Queue<T, CAPACITY> {
    fn default() -> Self {
        Self::new()
    }
}