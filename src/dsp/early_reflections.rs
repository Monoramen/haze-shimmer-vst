use super::delay_line::DelayLine;

const NUM_TAPS: usize = 8;

// L channel: отражения от левой/передней стены.
const TAP_DELAYS_L_MS: [f32; NUM_TAPS] = [5.0, 11.3, 18.7, 27.4, 36.1, 47.8, 61.2, 76.5];
// R channel: сдвинуты на ~3 мс — имитирует другое расстояние до правой стены.
const TAP_DELAYS_R_MS: [f32; NUM_TAPS] = [8.1, 14.6, 22.0, 31.3, 40.7, 52.4, 65.9, 81.2];

// Экспоненциальное затухание: gain[i] = exp(-i * 0.18)
const TAP_GAINS: [f32; NUM_TAPS] = [
    0.835, 0.698, 0.583, 0.487, 0.407, 0.340, 0.284, 0.237,
];

pub struct EarlyReflections {
    buffer_l: DelayLine,
    buffer_r: DelayLine,
    sample_rate: f32,
}

impl EarlyReflections {
    pub fn new(sample_rate: f32) -> Self {
        let max_ms = TAP_DELAYS_R_MS[NUM_TAPS - 1] * 4.0;
        let max_samples = ((max_ms * 0.001 * sample_rate).ceil() as usize + 4).next_power_of_two();
        Self {
            buffer_l: DelayLine::with_min_capacity(max_samples),
            buffer_r: DelayLine::with_min_capacity(max_samples),
            sample_rate,
        }
    }

    pub fn reset(&mut self) {
        self.buffer_l.clear();
        self.buffer_r.clear();
    }

    /// Returns (early_l, early_r). Each channel has its own tap pattern for true stereo.
    #[inline]
    pub fn process(&mut self, input_l: f32, input_r: f32, time_scale: f32) -> (f32, f32) {
        self.buffer_l.write(input_l);
        self.buffer_r.write(input_r);

        let scale = time_scale.max(0.1);
        let mut out_l = 0.0;
        let mut out_r = 0.0;
        for i in 0..NUM_TAPS {
            let d_l = (TAP_DELAYS_L_MS[i] * 0.001 * self.sample_rate * scale).max(1.0);
            let d_r = (TAP_DELAYS_R_MS[i] * 0.001 * self.sample_rate * scale).max(1.0);
            out_l += self.buffer_l.read(d_l) * TAP_GAINS[i];
            out_r += self.buffer_r.read(d_r) * TAP_GAINS[i];
        }
        (out_l, out_r)
    }
}
