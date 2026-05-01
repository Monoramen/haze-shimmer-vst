use super::delay_line::DelayLine;
use super::filters::OnePoleLP;

const N: usize = 16;

// Mutually prime delay lengths (~15..89 ms). Scaled proportionally at other sample rates.
const BASE_DELAYS_MS: [f32; N] = [
    15.1, 18.7, 22.3, 27.4, 31.9, 36.1, 39.7, 43.3,
    47.9, 53.1, 58.7, 63.2, 68.5, 74.1, 80.3, 86.9,
];

pub struct FdnParams {
    pub time_ms: f32,
    pub feedback: f32,
    /// 0..1 — controls LP cutoff inside each delay line (0=bright, 1=dark).
    pub diffusion: f32,
    pub sample_rate: f32,
}

pub struct Fdn {
    delays: [DelayLine; N],
    lpf: [OnePoleLP; N],
    state: [f32; N],
}

impl Fdn {
    pub fn new(sample_rate: f32) -> Self {
        let max_ms = 2200.0_f32;
        let max_samples = ((max_ms * 0.001 * sample_rate).ceil() as usize + 4).next_power_of_two();
        Self {
            delays: std::array::from_fn(|_| DelayLine::with_min_capacity(max_samples)),
            lpf: std::array::from_fn(|_| OnePoleLP::new()),
            state: [0.0; N],
        }
    }

    pub fn reset(&mut self) {
        for d in &mut self.delays {
            d.clear();
        }
        for f in &mut self.lpf {
            f.reset();
        }
        self.state = [0.0; N];
    }

    pub fn configure(&mut self, p: &FdnParams) {
        // LP cutoff: diffusion=0 → 18 kHz (bright), diffusion=1 → 800 Hz (dark).
        let lp_hz = 18_000.0_f32 * (1.0 - p.diffusion * 0.956);
        for f in &mut self.lpf {
            f.set_cutoff(lp_hz, p.sample_rate);
        }
    }

    /// Process one stereo sample pair.
    /// L output = sum of even lines, R output = sum of odd lines.
    #[inline]
    pub fn process(&mut self, input_l: f32, input_r: f32, p: &FdnParams) -> (f32, f32) {
        let sr = p.sample_rate;
        let fb = p.feedback.clamp(0.0, 0.95);

        // Scale delay lengths by tail_time relative to the base (480 ms reference).
        let time_scale = p.time_ms / 480.0;

        // Read from each delay line.
        let mut v = [0.0f32; N];
        for i in 0..N {
            let d_samples = (BASE_DELAYS_MS[i] * 0.001 * sr * time_scale).max(1.0);
            v[i] = self.delays[i].read(d_samples);
        }

        // Fast Hadamard Transform 16×16 — only additions, no multiplications.
        // Normalised by 1/sqrt(16) applied as a single scale after the butterfly.
        fht16(&mut v);
        let norm = 1.0 / (N as f32).sqrt();
        for x in &mut v {
            *x *= norm;
        }

        // Apply LP filter (tonal absorption) and feedback gain.
        for i in 0..N {
            v[i] = self.lpf[i].process(v[i]) * fb;
        }

        // Inject stereo input: even lines get L, odd lines get R.
        for i in 0..N {
            if i % 2 == 0 {
                v[i] += input_l;
            } else {
                v[i] += input_r;
            }
        }

        // Write back into each delay line.
        for i in 0..N {
            self.delays[i].write(v[i]);
            self.state[i] = v[i];
        }

        // Mix output: L = even lines, R = odd lines.
        let scale = 1.0 / (N / 2) as f32;
        let out_l = (v[0] + v[2] + v[4] + v[6] + v[8] + v[10] + v[12] + v[14]) * scale;
        let out_r = (v[1] + v[3] + v[5] + v[7] + v[9] + v[11] + v[13] + v[15]) * scale;

        (out_l, out_r)
    }
}

/// In-place Fast Hadamard Transform for N=16, 4 butterfly stages.
#[inline]
fn fht16(v: &mut [f32; 16]) {
    // Stage 1: stride 1
    for i in (0..16).step_by(2) {
        let a = v[i];
        let b = v[i + 1];
        v[i] = a + b;
        v[i + 1] = a - b;
    }
    // Stage 2: stride 2
    for i in (0..16).step_by(4) {
        let a0 = v[i];     let a1 = v[i + 1];
        let b0 = v[i + 2]; let b1 = v[i + 3];
        v[i]     = a0 + b0; v[i + 1] = a1 + b1;
        v[i + 2] = a0 - b0; v[i + 3] = a1 - b1;
    }
    // Stage 3: stride 4
    for i in (0..16).step_by(8) {
        let a0 = v[i];     let a1 = v[i + 1];
        let a2 = v[i + 2]; let a3 = v[i + 3];
        let b0 = v[i + 4]; let b1 = v[i + 5];
        let b2 = v[i + 6]; let b3 = v[i + 7];
        v[i]     = a0 + b0; v[i + 1] = a1 + b1;
        v[i + 2] = a2 + b2; v[i + 3] = a3 + b3;
        v[i + 4] = a0 - b0; v[i + 5] = a1 - b1;
        v[i + 6] = a2 - b2; v[i + 7] = a3 - b3;
    }
    // Stage 4: stride 8
    let a0 = v[0];  let a1 = v[1];  let a2 = v[2];  let a3 = v[3];
    let a4 = v[4];  let a5 = v[5];  let a6 = v[6];  let a7 = v[7];
    let b0 = v[8];  let b1 = v[9];  let b2 = v[10]; let b3 = v[11];
    let b4 = v[12]; let b5 = v[13]; let b6 = v[14]; let b7 = v[15];
    v[0]  = a0 + b0; v[1]  = a1 + b1; v[2]  = a2 + b2; v[3]  = a3 + b3;
    v[4]  = a4 + b4; v[5]  = a5 + b5; v[6]  = a6 + b6; v[7]  = a7 + b7;
    v[8]  = a0 - b0; v[9]  = a1 - b1; v[10] = a2 - b2; v[11] = a3 - b3;
    v[12] = a4 - b4; v[13] = a5 - b5; v[14] = a6 - b6; v[15] = a7 - b7;
}
