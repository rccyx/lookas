pub struct SharedBuf {
    data: Vec<f32>,
    write_idx: usize,
    filled: bool,
}

impl SharedBuf {
    pub fn new(cap: usize) -> Self {
        Self {
            data: vec![0.0; cap],
            write_idx: 0,
            filled: false,
        }
    }

    #[inline]
    pub fn push(&mut self, x: f32) {
        self.data[self.write_idx] = x;
        self.write_idx = (self.write_idx + 1) % self.data.len();
        if self.write_idx == 0 {
            self.filled = true;
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        if self.filled {
            self.data.len()
        } else {
            self.write_idx
        }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        !self.filled && self.write_idx == 0
    }

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
                out.copy_from_slice(&self.data[start..start + n]);
            } else {
                let first = cap - start;
                out[..first].copy_from_slice(&self.data[start..cap]);
                out[first..]
                    .copy_from_slice(&self.data[..(n - first)]);
            }
        } else {
            let start = self.write_idx - n;
            out.copy_from_slice(&self.data[start..self.write_idx]);
        }

        true
    }

    pub fn latest(&self) -> Vec<f32> {
        let len = if self.filled {
            self.data.len()
        } else {
            self.write_idx
        };

        if len == 0 {
            return Vec::new();
        }

        let mut result = Vec::with_capacity(len);

        if self.filled {
            unsafe {
                let ptr = result.as_mut_ptr();
                let first_chunk = self.data.len() - self.write_idx;
                std::ptr::copy_nonoverlapping(
                    self.data.as_ptr().add(self.write_idx),
                    ptr,
                    first_chunk,
                );
                std::ptr::copy_nonoverlapping(
                    self.data.as_ptr(),
                    ptr.add(first_chunk),
                    self.write_idx,
                );
                result.set_len(len);
            }
        } else {
            result.extend_from_slice(&self.data[..self.write_idx]);
        }

        result
    }
}
