use super::delay_line::DelayLine;
use super::filters::{OnePoleHP, OnePoleLP, soft_clip};

#[allow(dead_code)]
pub const MODE_STEREO: i32 = 0;
pub const MODE_PING_PONG: i32 = 1;
pub const MODE_MID_SIDE: i32 = 2;

pub struct GrainDelay {
    line_l: DelayLine,
    line_r: DelayLine,
    fb_l: f32,
    fb_r: f32,
    hpf_l: OnePoleHP,
    hpf_r: OnePoleHP,
    lpf_l: OnePoleLP,
    lpf_r: OnePoleLP,
    /// Smoothed duck envelope follower.
    duck_env: f32,
}

impl GrainDelay {
    pub fn new(sample_rate: f32) -> Self {
        let max_samples =
            ((2000.0_f32 * 0.001 * sample_rate).ceil() as usize + 4).next_power_of_two();
        Self {
            line_l: DelayLine::with_min_capacity(max_samples),
            line_r: DelayLine::with_min_capacity(max_samples),
            fb_l: 0.0,
            fb_r: 0.0,
            hpf_l: OnePoleHP::new(),
            hpf_r: OnePoleHP::new(),
            lpf_l: OnePoleLP::new(),
            lpf_r: OnePoleLP::new(),
            duck_env: 0.0,
        }
    }

    pub fn reset(&mut self) {
        self.line_l.clear();
        self.line_r.clear();
        self.fb_l = 0.0;
        self.fb_r = 0.0;
        self.hpf_l.reset();
        self.hpf_r.reset();
        self.lpf_l.reset();
        self.lpf_r.reset();
        self.duck_env = 0.0;
    }

    #[inline]
    #[allow(clippy::too_many_arguments)]
    pub fn process(
        &mut self,
        in_l: f32,
        in_r: f32,
        time_ms: f32,
        feedback: f32,
        mode: i32,
        sample_rate: f32,
        mix: f32,
        detune_cents: f32,
        hpf_hz: f32,
        lpf_hz: f32,
        duck: f32,
    ) -> (f32, f32) {
        self.hpf_l.set_cutoff(hpf_hz, sample_rate);
        self.hpf_r.set_cutoff(hpf_hz, sample_rate);
        self.lpf_l.set_cutoff(lpf_hz, sample_rate);
        self.lpf_r.set_cutoff(lpf_hz, sample_rate);

        // Detune: L reads at base time, R reads at slightly different time.
        // detune_cents → pitch ratio → time ratio (pitch up = shorter delay).
        let detune_ratio = 2.0_f32.powf(detune_cents / 1200.0);
        let time_r_ms = time_ms / detune_ratio;

        let (raw_l, raw_r) = match mode {
            MODE_PING_PONG => self.process_ping_pong(in_l, in_r, time_ms, time_r_ms, feedback, sample_rate),
            MODE_MID_SIDE => self.process_mid_side(in_l, in_r, time_ms, feedback, sample_rate),
            _ => self.process_stereo(in_l, in_r, time_ms, time_r_ms, feedback, sample_rate),
        };

        // Tone shaping on delay output.
        let filt_l = self.hpf_l.process(self.lpf_l.process(raw_l));
        let filt_r = self.hpf_r.process(self.lpf_r.process(raw_r));

        // Duck: envelope follower on input suppresses delay output.
        let input_level = in_l.abs().max(in_r.abs());
        let attack = 0.9997;  // ~10 ms release
        let release = 0.9999; // ~20 ms release
        self.duck_env = if input_level > self.duck_env {
            self.duck_env * attack + input_level * (1.0 - attack)
        } else {
            self.duck_env * release + input_level * (1.0 - release)
        };
        let duck_gain = 1.0 - duck * self.duck_env.min(1.0);

        let wet_l = filt_l * duck_gain;
        let wet_r = filt_r * duck_gain;

        // Mix dry/wet.
        let out_l = in_l * (1.0 - mix) + wet_l * mix;
        let out_r = in_r * (1.0 - mix) + wet_r * mix;
        (out_l, out_r)
    }

    #[inline]
    fn process_stereo(
        &mut self,
        in_l: f32,
        in_r: f32,
        time_l_ms: f32,
        time_r_ms: f32,
        feedback: f32,
        sample_rate: f32,
    ) -> (f32, f32) {
        let d_l = (time_l_ms * 0.001 * sample_rate).max(1.0);
        let d_r = (time_r_ms * 0.001 * sample_rate).max(1.0);
        let write_l = soft_clip(in_l + self.fb_l);
        let write_r = soft_clip(in_r + self.fb_r);
        self.line_l.write(write_l);
        self.line_r.write(write_r);
        let out_l = self.line_l.read(d_l);
        let out_r = self.line_r.read(d_r);
        self.fb_l = out_l * feedback;
        self.fb_r = out_r * feedback;
        (out_l, out_r)
    }

    #[inline]
    fn process_ping_pong(
        &mut self,
        in_l: f32,
        in_r: f32,
        time_l_ms: f32,
        time_r_ms: f32,
        feedback: f32,
        sample_rate: f32,
    ) -> (f32, f32) {
        let d_l = (time_l_ms * 0.001 * sample_rate).max(1.0);
        let d_r = (time_r_ms * 0.001 * sample_rate).max(1.0);
        let write_l = soft_clip(in_l + self.fb_l);
        let write_r = soft_clip(in_r + self.fb_r);
        self.line_l.write(write_l);
        self.line_r.write(write_r);
        let out_l = self.line_l.read(d_l);
        let out_r = self.line_r.read(d_r);
        self.fb_l = out_r * feedback;
        self.fb_r = out_l * feedback;
        (out_l, out_r)
    }

    #[inline]
    fn process_mid_side(
        &mut self,
        in_l: f32,
        in_r: f32,
        time_ms: f32,
        feedback: f32,
        sample_rate: f32,
    ) -> (f32, f32) {
        let mid = (in_l + in_r) * 0.5;
        let side = (in_l - in_r) * 0.5;

        let d_mid = (time_ms * 0.001 * sample_rate).max(1.0);
        let d_side = (time_ms * 0.75 * 0.001 * sample_rate).max(1.0);

        let write_mid = soft_clip(mid + self.fb_l);
        let write_side = soft_clip(side + self.fb_r);
        self.line_l.write(write_mid);
        self.line_r.write(write_side);

        let out_mid = self.line_l.read(d_mid);
        let out_side = self.line_r.read(d_side);

        self.fb_l = out_mid * feedback;
        self.fb_r = out_side * feedback;

        let out_l = out_mid + out_side;
        let out_r = out_mid - out_side;
        (out_l, out_r)
    }
}
