#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct Bitmap<T>(T);

pub struct BitmapIter<T> {
    mask: T,
}

pub struct BitmapCircularIter<T> {
    upper: T,
    lower: T,
}

macro_rules! impl_bitmap {
    ($t:ty) => {
        #[allow(dead_code)] // if not all of implementations are used
        impl Bitmap<$t> {
            pub const SIZE: usize = <$t>::BITS as usize;

            #[inline]
            pub const fn new() -> Self { Self(0) }

            #[inline]
            pub const fn from(val: $t) -> Self { Self(val) }

            #[inline]
            pub const fn raw(&self) -> $t { self.0 }

            #[inline]
            pub fn set(&mut self, bit: usize) { self.0 |= (1 as $t) << bit; }

            #[inline]
            pub fn clear(&mut self, bit: usize) { self.0 &= !((1 as $t) << bit); }

            #[inline]
            pub fn is_empty(&self) -> bool { self.0 == 0 }

            #[inline]
            pub fn is_set(&self, bit: usize) -> bool { (self.0 & ((1 as $t) << bit)) != 0 }

            #[inline]
            pub fn ones(&self) -> usize { self.0.count_ones() as usize }

            #[inline]
            pub fn first_unset(&self) -> Option<usize> {
                let inverted = !self.0;
                if inverted == 0 {
                    None // all bits set to 1
                } else {
                    Some(inverted.trailing_zeros() as usize)
                }
            }

            #[inline]
            pub fn set_range(&mut self, start: usize, len: usize) {
                let mask = if len == Self::SIZE {
                    <$t>::MAX
                } else {
                    ((1 as $t) << len).wrapping_sub(1)
                };
                self.0 |= mask << start;
            }

            #[inline]
            pub fn clear_range(&mut self, start: usize, len: usize) {
                let mask = if len == Self::SIZE {
                    <$t>::MAX
                } else {
                    ((1 as $t) << len).wrapping_sub(1)
                };
                self.0 &= !(mask << start);
            }

            #[inline]
            pub fn find_first_fit(&self, len: usize) -> Option<usize> {
                if len == 0 || len > Self::SIZE { return None; }

                let search_mask = if len == Self::SIZE {
                    <$t>::MAX
                } else {
                    ((1 as $t) << len).wrapping_sub(1)
                };

                for i in 0..=(Self::SIZE - len) {
                    if (self.0 & (search_mask << i)) == 0 {
                        return Some(i);
                    }
                }
                None
            }

            #[inline]
            pub fn iter(self) -> BitmapIter<$t> {
                BitmapIter { mask: self.0 }
            }

            #[inline]
            pub fn iter_from(self, start_bit: usize) -> BitmapCircularIter<$t> {
                let mask_lower = ((1 as $t) << start_bit).wrapping_sub(1);
                BitmapCircularIter {
                    upper: self.0 & !mask_lower,
                    lower: self.0 & mask_lower,
                }
            }
        }

        impl Iterator for BitmapIter<$t> {
            type Item = usize;

            #[inline(always)]
            fn next(&mut self) -> Option<Self::Item> {
                if self.mask == 0 { return None; }
                let bit = self.mask.trailing_zeros() as usize;
                self.mask &= !((1 as $t) << bit);
                Some(bit)
            }
        }

        impl Iterator for BitmapCircularIter<$t> {
            type Item = usize;

            #[inline(always)]
            fn next(&mut self) -> Option<Self::Item> {
                if self.upper != 0 {
                    let bit = self.upper.trailing_zeros() as usize;
                    self.upper &= !((1 as $t) << bit);
                    return Some(bit);
                }
                if self.lower != 0 {
                    let bit = self.lower.trailing_zeros() as usize;
                    self.lower &= !((1 as $t) << bit);
                    return Some(bit);
                }
                None
            }
        }
    };
}

impl_bitmap!(u8);
impl_bitmap!(u16);
impl_bitmap!(u32);
impl_bitmap!(u64);
impl_bitmap!(u128);