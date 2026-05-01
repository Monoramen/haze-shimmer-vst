use super::envelope::EnvelopeShape;
use super::grain::Grain;
use super::ring_buffer::RingBuffer;
use super::rng::Rng;

pub const MAX_GRAINS: usize = 64;

pub struct GrainParams {
    pub size_samples: f64,
    /// Pitch shift in semitones. speed = 2^(st/12).
    pub semitones: f64,
    pub density_hz: f32,
    pub envelope: EnvelopeShape,
    pub position_jitter: f32,
    /// Fine detuning in cents (±). Kept small to preserve tonal clarity.
    pub pitch_jitter_cents: f32,
    pub sample_rate: f32,
}

impl GrainParams {
    #[inline]
    pub fn base_speed(&self) -> f64 {
        2.0_f64.powf(self.semitones / 12.0)
    }
}

pub struct GrainPool {
    grains: [Grain; MAX_GRAINS],
    samples_until_next: f64,
    rng: Rng,
    spawn_index: u64,
}

impl GrainPool {
    pub fn new() -> Self {
        Self {
            grains: [Grain::INACTIVE; MAX_GRAINS],
            samples_until_next: 0.0,
            rng: Rng::new(0xC0FFEE),
            spawn_index: 0,
        }
    }

    pub fn reset(&mut self) {
        for g in &mut self.grains {
            *g = Grain::INACTIVE;
        }
        self.samples_until_next = 0.0;
        self.spawn_index = 0;
    }

    fn spawn(&mut self, params: &GrainParams) {
        let slot = self.grains.iter_mut().find(|g| !g.active);
        let Some(slot) = slot else { return };

        // Apply pitch jitter first so start_delay tracks the actual speed
        // used by this grain — keeps the overlap grid aligned.
        let cents = self.rng.next_bipolar() * params.pitch_jitter_cents;
        let speed_mult = 2.0_f64.powf(cents as f64 / 1200.0);
        let speed = params.base_speed() * speed_mult;

        let base_delay = speed * params.size_samples;

        // Phase-offset overlapping grains so they don't read the same buffer
        // position in lockstep. The offset cycles within one grain size:
        // (spawn_index mod overlap_count) * period. This keeps the offset
        // bounded and independent of how long the pool has been running,
        // so changing Size or Density doesn't snowball into a big jump.
        let (overlap_count, period) = Self::spawn_period_info(params);
        let slot_idx = (self.spawn_index % overlap_count as u64) as f64;
        let phase_offset = period * slot_idx;
        self.spawn_index = self.spawn_index.wrapping_add(1);

        // Symmetric position jitter, clamped so the grain never starts
        // closer than size_samples + 2 from the write head (need 2 samples
        // lookahead for cubic interp).
        let jitter_range = params.position_jitter as f64 * params.size_samples;
        let jitter = self.rng.next_bipolar() as f64 * jitter_range;
        let min_delay = params.size_samples + 2.0;
        let start_delay = (base_delay + phase_offset + jitter).max(min_delay);

        // Coherent-sum normalization: overlapping grains read correlated
        // samples, so amplitude adds linearly (1/N), not as 1/√N.
        let gain = (1.0 / overlap_count) as f32;

        *slot = Grain::new(
            start_delay,
            params.size_samples,
            speed,
            params.envelope,
            gain,
        );
    }

    /// Spawn period driven directly by the user's Density parameter.
    /// Returns (overlap_count, period_in_samples).
    ///
    /// `period` = SR / density_hz — honors what the user asks for, so low
    /// density yields audible gaps between grains.
    ///
    /// `overlap_count` = size_samples / period. May be fractional and may
    /// be < 1.0 (sparse grains with silence between them). Used only for
    /// amplitude normalization.
    #[inline]
    fn spawn_period_info(params: &GrainParams) -> (f64, f64) {
        let sr = params.sample_rate as f64;
        let period = (sr / params.density_hz.max(0.01) as f64).max(1.0);
        let overlap_count = (params.size_samples / period).max(1.0);
        (overlap_count, period)
    }

    #[inline]
    fn spawn_period(params: &GrainParams) -> f64 {
        Self::spawn_period_info(params).1
    }

    /// Process one sample: advance scheduler and mix active grains.
    #[inline]
    pub fn process_sample(&mut self, params: &GrainParams, buffer: &RingBuffer) -> f32 {
        if self.samples_until_next <= 0.0 {
            self.spawn(params);
            self.samples_until_next += Self::spawn_period(params);
        }
        self.samples_until_next -= 1.0;

        let mut out = 0.0;
        for g in &mut self.grains {
            out += g.tick(buffer);
        }
        out
    }
}
