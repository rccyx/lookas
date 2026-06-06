pub struct SharedBuf {
    data: Vec<f32>,
    write_idx: usize,
    filled: bool,
}

impl SharedBuf {
    #[must_use]
    pub fn new(cap: usize) -> Self {
        let cap = cap.checked_next_power_of_two().unwrap_or(0);
        Self {
            data: vec![0.0; cap],
            write_idx: 0,
            filled: false,
        }
    }

    #[inline]
    #[allow(clippy::arithmetic_side_effects)]
    pub fn push(&mut self, x: f32) {
        let cap = self.data.len();
        if cap == 0 {
            return;
        }

        if let Some(slot) = self.data.get_mut(self.write_idx) {
            *slot = x;
        }

        // `new` rounds every nonzero request to a power-of-two capacity.
        let mask = cap - 1;
        self.write_idx = (self.write_idx + 1) & mask;
        if self.write_idx == 0 {
            self.filled = true;
        }
    }

    #[inline]
    #[must_use]
    pub fn len(&self) -> usize {
        if self.filled {
            self.data.len()
        } else {
            self.write_idx
        }
    }

    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        !self.filled && self.write_idx == 0
    }

    #[allow(clippy::arithmetic_side_effects)]
    pub fn copy_last_n_into(
        &self,
        n: usize,
        out: &mut Vec<f32>,
    ) -> bool {
        if n == 0 {
            out.clear();
            return true;
        }

        let cap = self.data.len();
        let len = self.len();
        if len < n {
            return false;
        }

        out.resize(n, 0.0);

        if self.filled {
            let start = (self.write_idx + cap - n) % cap;
            if start + n <= cap {
                let Some(src) = self.data.get(start..start + n)
                else {
                    return false;
                };
                out.copy_from_slice(src);
            } else {
                let first = cap - start;
                let (head, tail) = out.split_at_mut(first);
                let (Some(first_src), Some(second_src)) = (
                    self.data.get(start..cap),
                    self.data.get(..(n - first)),
                ) else {
                    return false;
                };
                head.copy_from_slice(first_src);
                tail.copy_from_slice(second_src);
            }
        } else {
            let start = self.write_idx - n;
            let Some(src) = self.data.get(start..self.write_idx)
            else {
                return false;
            };
            out.copy_from_slice(src);
        }

        true
    }

    #[must_use]
    pub fn latest(&self) -> Vec<f32> {
        let len = self.len();

        if len == 0 {
            return Vec::new();
        }

        let mut result = Vec::with_capacity(len);

        if self.filled {
            if let Some(src) = self.data.get(self.write_idx..) {
                result.extend_from_slice(src);
            }
            if let Some(src) = self.data.get(..self.write_idx) {
                result.extend_from_slice(src);
            }
        } else if let Some(src) = self.data.get(..self.write_idx) {
            result.extend_from_slice(src);
        }

        result
    }
}
