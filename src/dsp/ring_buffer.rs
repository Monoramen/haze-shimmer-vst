pub struct RingBuffer {
    data: Vec<f32>,
    write_pos: usize,
    mask: usize,
}

impl RingBuffer {
    pub fn with_min_capacity(min_cap: usize) -> Self {
        let cap = min_cap.max(2).next_power_of_two();
        Self {
            data: vec![0.0; cap],
            write_pos: 0,
            mask: cap - 1,
        }
    }

    #[inline]
    pub fn write(&mut self, sample: f32) {
        self.write_pos = (self.write_pos + 1) & self.mask;
        self.data[self.write_pos] = sample;
    }

    #[inline]
    fn get_safe(&self, idx: isize) -> f32 {
        self.data[(idx as usize) & self.mask]
    }

    pub fn read_cubic(&self, delay_samples: f64) -> f32 {
        let read_pos = self.write_pos as f64 - delay_samples;
        let i = read_pos.floor() as isize;
        let frac = (read_pos - i as f64) as f32;

        let s0 = self.get_safe(i - 1);
        let s1 = self.get_safe(i);
        let s2 = self.get_safe(i + 1);
        let s3 = self.get_safe(i + 2);

        // Catmull-Rom cubic interpolation
        let a = -0.5 * s0 + 1.5 * s1 - 1.5 * s2 + 0.5 * s3;
        let b = s0 - 2.5 * s1 + 2.0 * s2 - 0.5 * s3;
        let c = -0.5 * s0 + 0.5 * s2;
        let d = s1;

        a * frac.powi(3) + b * frac.powi(2) + c * frac + d
    }

    pub fn clear(&mut self) {
        self.data.fill(0.0);
        self.write_pos = 0;
    }
}
