use super::allpass::Allpass;
use super::early_reflections::EarlyReflections;
use super::fdn::{Fdn, FdnParams};
use super::filters::{OnePoleHP, OnePoleLP};

pub struct ReverbParams {
    pub time_ms: f32,
    pub feedback: f32,
    pub diffusion: f32,
    #[allow(dead_code)]
    pub modulation_ms: f32, // reserved for future LFO modulation depth
    pub hpf_hz: f32,
    pub lpf_hz: f32,
    pub sample_rate: f32,
}

pub struct Reverb {
    fdn: Fdn,
    early: EarlyReflections,
    /// Allpass decorrelator on the R channel input (~0.5 ms).
    decorr_r: Allpass,
    hpf_l: OnePoleHP,
    hpf_r: OnePoleHP,
    lpf_l: OnePoleLP,
    lpf_r: OnePoleLP,
}

impl Reverb {
    pub fn new(sample_rate: f32) -> Self {
        // ~0.5 ms decorrelation delay at 48 kHz = 24 samples
        let decorr_samples = ((0.5e-3 * sample_rate).ceil() as usize).max(2);
        let mut decorr_r = Allpass::new(decorr_samples * 2);
        decorr_r.set_delay(decorr_samples as f32);
        decorr_r.set_gain(0.5);

        Self {
            fdn: Fdn::new(sample_rate),
            early: EarlyReflections::new(sample_rate),
            decorr_r,
            hpf_l: OnePoleHP::new(),
            hpf_r: OnePoleHP::new(),
            lpf_l: OnePoleLP::new(),
            lpf_r: OnePoleLP::new(),
        }
    }

    pub fn reset(&mut self) {
        self.fdn.reset();
        self.early.reset();
        self.decorr_r.reset();
        self.hpf_l.reset();
        self.hpf_r.reset();
        self.lpf_l.reset();
        self.lpf_r.reset();
    }

    pub fn configure(&mut self, p: &ReverbParams) {
        let fp = FdnParams {
            time_ms: p.time_ms,
            feedback: p.feedback,
            diffusion: p.diffusion,
            sample_rate: p.sample_rate,
        };
        self.fdn.configure(&fp);
        self.hpf_l.set_cutoff(p.hpf_hz, p.sample_rate);
        self.hpf_r.set_cutoff(p.hpf_hz, p.sample_rate);
        self.lpf_l.set_cutoff(p.lpf_hz, p.sample_rate);
        self.lpf_r.set_cutoff(p.lpf_hz, p.sample_rate);
    }

    /// Returns (tail_l, tail_r, early_l, early_r).
    #[inline]
    pub fn process(&mut self, input_l: f32, input_r: f32, p: &ReverbParams) -> (f32, f32, f32, f32) {
        let time_scale = p.time_ms / 480.0;

        // Early reflections: true stereo — L and R have independent tap patterns.
        let (early_l, early_r) = self.early.process(input_l, input_r, time_scale);

        // Decorrelate R channel before entering FDN.
        let in_r_decorr = self.decorr_r.process(input_r);

        let fp = FdnParams {
            time_ms: p.time_ms,
            feedback: p.feedback,
            diffusion: p.diffusion,
            sample_rate: p.sample_rate,
        };
        let (raw_l, raw_r) = self.fdn.process(input_l, in_r_decorr, &fp);

        // Tone shaping on FDN output.
        let tail_l = self.hpf_l.process(self.lpf_l.process(raw_l));
        let tail_r = self.hpf_r.process(self.lpf_r.process(raw_r));

        (tail_l, tail_r, early_l, early_r)
    }
}
