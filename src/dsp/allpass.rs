use super::delay_line::DelayLine;

/// Schroeder allpass: y = -g*x + z, where z = delayed(x + g*y)
pub struct Allpass {
    line: DelayLine,
    delay_samples: f32,
    gain: f32,
}

impl Allpass {
    pub fn new(max_delay_samples: usize) -> Self {
        Self {
            line: DelayLine::with_min_capacity(max_delay_samples.max(2)),
            delay_samples: max_delay_samples as f32 * 0.5,
            gain: 0.5,
        }
    }

    pub fn reset(&mut self) {
        self.line.clear();
    }

    pub fn set_delay(&mut self, delay_samples: f32) {
        let cap = self.line.capacity() as f32;
        self.delay_samples = delay_samples.clamp(1.0, cap - 2.0);
    }

    pub fn set_gain(&mut self, gain: f32) {
        self.gain = gain.clamp(-0.95, 0.95);
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let delayed = self.line.read(self.delay_samples);
        let v = x + delayed * self.gain;
        self.line.write(v);
        delayed - v * self.gain
    }
}

/// Four cascaded allpasses for diffusion (kept for potential future use).
#[allow(dead_code)]
pub struct DiffusionChain {
    stages: [Allpass; 4],
}

#[allow(dead_code)]
impl DiffusionChain {
    pub fn new(sample_rate: f32) -> Self {
        // Prime-ish delay lengths in ms, scaled to sample rate.
        let lengths_ms = [5.3, 7.7, 11.1, 17.3];
        let stages = [
            Allpass::new((lengths_ms[0] * 0.001 * sample_rate).ceil() as usize * 2),
            Allpass::new((lengths_ms[1] * 0.001 * sample_rate).ceil() as usize * 2),
            Allpass::new((lengths_ms[2] * 0.001 * sample_rate).ceil() as usize * 2),
            Allpass::new((lengths_ms[3] * 0.001 * sample_rate).ceil() as usize * 2),
        ];
        let mut chain = Self { stages };
        chain.configure(sample_rate, 0.5);
        chain
    }

    pub fn reset(&mut self) {
        for s in &mut self.stages {
            s.reset();
        }
    }

    /// amount in [0, 1] — blend between bypass and full diffusion gain (~0.7).
    pub fn configure(&mut self, sample_rate: f32, amount: f32) {
        let amt = amount.clamp(0.0, 1.0);
        let lengths_ms = [5.3, 7.7, 11.1, 17.3];
        let g = 0.7 * amt;
        for (i, stage) in self.stages.iter_mut().enumerate() {
            stage.set_delay(lengths_ms[i] * 0.001 * sample_rate);
            stage.set_gain(g);
        }
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let mut y = x;
        for s in &mut self.stages {
            y = s.process(y);
        }
        y
    }
}
