pub struct DelayLine {
    data: Vec<f32>,
    write_pos: usize,
    mask: usize,
}

impl DelayLine {
    pub fn with_min_capacity(min_cap: usize) -> Self {
        let cap = min_cap.next_power_of_two().max(4);
        Self {
            data: vec![0.0; cap],
            write_pos: 0,
            mask: cap - 1,
        }
    }

    pub fn clear(&mut self) {
        self.data.fill(0.0);
        self.write_pos = 0;
    }

    pub fn capacity(&self) -> usize {
        self.data.len()
    }

    #[inline]
    pub fn write(&mut self, x: f32) {
        self.data[self.write_pos] = x;
        self.write_pos = (self.write_pos + 1) & self.mask;
    }

    /// Fractional read using Catmull-Rom cubic interpolation.
    /// `delay_samples` must be in [1, capacity - 2].
    #[inline]
    pub fn read(&self, delay_samples: f32) -> f32 {
        let cap = self.data.len() as f32;
        let d = delay_samples.clamp(1.0, cap - 2.0);
        let int_part = d as usize;
        let frac = d - int_part as f32;

        // All indexing via & mask — no division or rem_euclid.
        let i1 = self.write_pos.wrapping_sub(int_part) & self.mask;
        let i0 = i1.wrapping_sub(1) & self.mask;
        let i2 = (i1 + 1) & self.mask;
        let i3 = (i1 + 2) & self.mask;

        let s0 = self.data[i0];
        let s1 = self.data[i1];
        let s2 = self.data[i2];
        let s3 = self.data[i3];

        // Catmull-Rom cubic interpolation (same as RingBuffer::read_cubic).
        let a = -0.5 * s0 + 1.5 * s1 - 1.5 * s2 + 0.5 * s3;
        let b = s0 - 2.5 * s1 + 2.0 * s2 - 0.5 * s3;
        let c = -0.5 * s0 + 0.5 * s2;
        let d2 = s1;
        a * frac * frac * frac + b * frac * frac + c * frac + d2
    }
}
