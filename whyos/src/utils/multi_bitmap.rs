use super::Bitmap;

#[derive(Clone, Copy)]
pub struct MultiBitmap<const N: usize> {
    words: [Bitmap<u64>; N],
}

#[allow(dead_code)]
impl<const N: usize> MultiBitmap<N> {
    pub const TOTAL_BITS: usize = N * 64;

    #[inline]
    pub const fn new() -> Self {
        Self { words: [Bitmap::<u64>::new(); N] }
    }

    #[inline]
    const fn loc(idx: usize) -> (usize, usize) {
        (idx / 64, idx % 64)
    }

    #[inline]
    pub fn is_set(&self, idx: usize) -> bool {
        let (w, b) = Self::loc(idx);
        self.words[w].is_set(b)
    }

    #[inline]
    pub fn set(&mut self, idx: usize) {
        let (w, b) = Self::loc(idx);
        self.words[w].set(b);
    }

    #[inline]
    pub fn clear(&mut self, idx: usize) {
        let (w, b) = Self::loc(idx);
        self.words[w].clear(b);
    }

    #[inline]
    pub fn set_range(&mut self, start: usize, len: usize) {
        self.range_op(start, len, true);
    }

    #[inline]
    pub fn clear_range(&mut self, start: usize, len: usize) {
        self.range_op(start, len, false);
    }

    // find the first run of 'len' consecutive free bits
    // common case where len < 64: tries single-word fit first O(1)
    pub fn find_first_fit(&self, len: usize) -> Option<usize> {
        if len == 0 || len > Self::TOTAL_BITS {
            return None;
        }

        let mut run_start = 0usize;
        let mut run_len   = 0usize;

        for (wi, word) in self.words.iter().enumerate() {
            let word_start = wi * 64;

            // entire word free
            if word.is_empty() {
                run_len += 64;
                if run_len >= len {
                    return Some(run_start);
                }
                continue;
            }

            // entire word occupied
            if word.is_full() {
                run_len   = 0;
                run_start = word_start + 64;
                continue;
            }

            // partial word

            // free bits at the start of this word - may complete a cross-word
            let tf = word.trailing_zeros();
            if run_len + tf >= len {
                return Some(run_start);
            }

            // len < 64 try a run entirely within this word
            if len <= 64 && let Some(bit) = word.find_first_fit(len) {
                return Some(word_start + bit);
            }

            // free bits at the end of this word - seed the next cross-word
            let lf = word.leading_zeros();
            run_len   = lf;
            run_start = word_start + 64 - lf;
        }

        if run_len >= len { Some(run_start) } else { None }
    }

    // applies set/clear across a possibly multi-word range
    fn range_op(&mut self, start: usize, len: usize, set: bool) {
        if len == 0 { return; }

        let end         = start + len; // exclusive
        let first_word  = start / 64;
        let last_word   = (end - 1) / 64;

        for wi in first_word..=last_word {
            let bit_start = if wi == first_word { start % 64 } else { 0 };
            let bit_end   = if wi == last_word  { (end - 1) % 64 + 1 } else { 64 };
            let bit_len   = bit_end - bit_start;

            if set {
                self.words[wi].set_range(bit_start, bit_len);
            } else {
                self.words[wi].clear_range(bit_start, bit_len);
            }
        }
    }
}