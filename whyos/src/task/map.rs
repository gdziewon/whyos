use super::TaskId;

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct TaskMap(pub(crate) u32); // because MAX_TASKS=32, each bit is representing a task

impl TaskMap {
    pub const fn new() -> Self {
        Self(0)
    }

    pub const fn from(value: u32) -> Self {
        Self(value)
    }

    #[inline]
    pub fn add(&mut self, tid: TaskId) {
        self.0 |= 1 << tid.id();
    }

    #[inline]
    pub fn remove(&mut self, tid: TaskId) {
        self.0 &= !(1 << tid.id());
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0 == 0
    }

    #[inline]
    pub fn is_set(&self, tid: TaskId) -> bool {
        (self.0 & (1 << tid.id())) != 0
    }

    #[inline]
    pub fn ones(&self) -> usize {
        self.0.count_ones() as usize
    }

    #[inline]
    pub fn iter(self) -> TaskMapIter {
        TaskMapIter { mask: self.0 }
    }

    #[inline]
    pub fn iter_from(self, start_bit: usize) -> TaskMapCircularIter { // start bit needs to be less then 32!
        let mask_lower = (1u32 << start_bit).wrapping_sub(1);

        TaskMapCircularIter {
            upper: self.0 & !mask_lower,
            lower: self.0 & mask_lower,
        }
    }
}

pub struct TaskMapIter {
    mask: u32,
}

impl Iterator for TaskMapIter {
    type Item = TaskId;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> {
        if self.mask == 0 {
            return None;
        }

        let tid = self.mask.trailing_zeros();

        self.mask &= !(1 << tid);

        // # SAFETY: We get indexes of ones in u32, it will be less then 32
        Some(unsafe { TaskId::new_unchecked(tid as usize) })
    }
}

pub struct TaskMapCircularIter {
    upper: u32,
    lower: u32,
}

impl Iterator for TaskMapCircularIter {
    type Item = TaskId;

    #[inline(always)]
    fn next(&mut self) -> Option<Self::Item> { // wrapping iter
        if self.upper != 0 {
            let tid = self.upper.trailing_zeros();
            self.upper &= !(1 << tid);
            return Some(unsafe { TaskId::new_unchecked(tid as usize) })
        }

        if self.lower != 0 {
            let tid = self.lower.trailing_zeros();
            self.lower &= !(1 << tid);
            return Some(unsafe { TaskId::new_unchecked(tid as usize) })
        }

        None
    }
}