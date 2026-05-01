use atomic_float::AtomicF32;
use nih_plug::prelude::*;
use std::sync::Arc;
use std::sync::atomic::Ordering;

mod dsp;
mod editor;
mod params;

use dsp::reverb::{Reverb, ReverbParams};
use dsp::envelope::EnvelopeShape;
use dsp::filters::{DcBlocker, soft_clip};
use dsp::grain_delay::GrainDelay;
use dsp::grain_pool::{GrainParams, GrainPool};
use dsp::ring_buffer::RingBuffer;
use params::ShimmerParams;

const MAX_SAMPLE_RATE: f32 = 192_000.0;
const BUFFER_SECONDS: f32 = 2.0;

// Division indices: 0=2/1, 1=3/1, 2=4/1, 3=1/1, 4=1/2, 5=1/2T, 6=1/4, 7=1/4T, 8=1/8, 9=1/8T, 10=1/16, 11=1/16T, 12=1/32, 13=1/32T
const DIVISION_BEATS: [f64; 14] = [
    8.0,          // 2/1
    12.0,         // 3/1
    16.0,         // 4/1
    4.0,          // 1/1
    2.0,          // 1/2
    4.0 / 3.0,    // 1/2T
    1.0,          // 1/4
    2.0 / 3.0,    // 1/4T
    0.5,          // 1/8
    1.0 / 3.0,    // 1/8T
    0.25,         // 1/16
    1.0 / 6.0,    // 1/16T
    0.125,        // 1/32
    1.0 / 12.0,   // 1/32T
];

#[inline]
fn division_to_ms(division: i32, tempo_bpm: f64) -> f32 {
    let beats = DIVISION_BEATS[division.clamp(0, 13) as usize];
    let beat_ms = 60_000.0 / tempo_bpm;
    (beats * beat_ms) as f32
}

#[derive(Clone, PartialEq)]
struct TailParamsCache {
    time_ms: f32,
    feedback: f32,
    diffusion: f32,
    modulation_ms: f32,
    hpf_hz: f32,
    lpf_hz: f32,
}

pub struct ShimmerGranular {
    params: Arc<ShimmerParams>,

    sample_rate: f32,

    buffer_l: RingBuffer,
    buffer_r: RingBuffer,
    pool_l: GrainPool,
    pool_r: GrainPool,

    reverb: Reverb,
    grain_delay: GrainDelay,

    dc_l: DcBlocker,
    dc_r: DcBlocker,

    fb_state_l: f32,
    fb_state_r: f32,

    tail_cache: Option<TailParamsCache>,
    shimmer_was_on: bool,
    tempo_bpm: f64,

    peak_meter: Arc<AtomicF32>,
    peak_decay_weight: f32,
}

impl Default for ShimmerGranular {
    fn default() -> Self {
        let cap = (MAX_SAMPLE_RATE * BUFFER_SECONDS) as usize;
        let sr = 48_000.0;
        Self {
            params: Arc::new(ShimmerParams::default()),
            sample_rate: sr,
            buffer_l: RingBuffer::with_min_capacity(cap),
            buffer_r: RingBuffer::with_min_capacity(cap),
            pool_l: GrainPool::new(),
            pool_r: GrainPool::new(),
            reverb: Reverb::new(sr),
            grain_delay: GrainDelay::new(sr),
            dc_l: DcBlocker::new(),
            dc_r: DcBlocker::new(),
            fb_state_l: 0.0,
            fb_state_r: 0.0,
            tail_cache: None,
            shimmer_was_on: true,
            tempo_bpm: 120.0,
            peak_meter: Arc::new(AtomicF32::new(util::MINUS_INFINITY_DB)),
            peak_decay_weight: 0.0,
        }
    }
}

impl Plugin for ShimmerGranular {
    const NAME: &'static str = "Shimmer Granular";
    const VENDOR: &'static str = "monoramens";
    const URL: &'static str = "";
    const EMAIL: &'static str = "monoramens@gmail.com";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        aux_input_ports: &[],
        aux_output_ports: &[],
        names: PortNames::const_default(),
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(self.params.clone())
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        self.peak_decay_weight = 0.25f32.powf(1.0 / (self.sample_rate * 0.1));
        true
    }

    fn reset(&mut self) {
        self.buffer_l.clear();
        self.buffer_r.clear();
        self.pool_l.reset();
        self.pool_r.reset();
        self.reverb.reset();
        self.grain_delay.reset();
        self.dc_l.reset();
        self.dc_r.reset();
        self.fb_state_l = 0.0;
        self.fb_state_r = 0.0;
        self.tail_cache = None;
        self.shimmer_was_on = true;
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        if let Some(bpm) = context.transport().tempo {
            self.tempo_bpm = bpm;
        }

        let mut peak = 0.0f32;

        for mut channel_samples in buffer.iter_samples() {
            let grain_size_ms = self.params.grain_size_ms.smoothed.next();
            let pitch_shift_st = self.params.pitch_shift_semitones.smoothed.next();
            let grain_density = self.params.grain_density.smoothed.next();
            let envelope = EnvelopeShape {
                attack: self.params.env_attack.value(),
                hold: self.params.env_hold.value(),
                attack_curve: self.params.env_attack_curve.value(),
                release_curve: self.params.env_release_curve.value(),
            };
            let pos_jitter = self.params.position_jitter.value();
            let pitch_jitter = self.params.pitch_jitter.value();

            let tail_time = self.params.tail_time_ms.smoothed.next();
            let tail_feedback = self.params.tail_feedback.smoothed.next();
            let tail_diffusion = self.params.tail_diffusion.smoothed.next();
            let tail_mod = self.params.tail_modulation_ms.smoothed.next();
            let tail_hpf = self.params.tail_hpf.smoothed.next();
            let tail_lpf = self.params.tail_lpf.smoothed.next();
            let regen = self.params.regen.smoothed.next();
            let dry_wet = self.params.dry_wet.smoothed.next();
            let out_gain = self.params.output_gain.smoothed.next();
            let gd_feedback = self.params.grain_delay_feedback.smoothed.next();
            let gd_mode = self.params.grain_delay_mode.value();
            let gd_sync = self.params.grain_delay_sync.value();
            let gd_time = if gd_sync {
                let division = self.params.grain_delay_division.value();
                division_to_ms(division, self.tempo_bpm)
            } else {
                self.params.grain_delay_time_ms.smoothed.next()
            };
            let gd_mix = self.params.gd_mix.smoothed.next();
            let gd_detune = self.params.gd_detune.smoothed.next();
            let gd_hpf = self.params.gd_hpf.smoothed.next();
            let gd_lpf = self.params.gd_lpf.smoothed.next();
            let gd_duck = self.params.gd_duck.smoothed.next();
            let tone_spread = self.params.tone_spread.smoothed.next();
            let output_width = self.params.output_width.smoothed.next();
            let tail_mix = self.params.tail_mix.smoothed.next();

            let size_samples = (grain_size_ms * 0.001 * self.sample_rate) as f64;
            let gp = GrainParams {
                size_samples,
                semitones: pitch_shift_st as f64,
                density_hz: grain_density,
                envelope,
                position_jitter: pos_jitter,
                pitch_jitter_cents: pitch_jitter,
                sample_rate: self.sample_rate,
            };

            let tp = ReverbParams {
                time_ms: tail_time,
                feedback: tail_feedback,
                diffusion: tail_diffusion,
                modulation_ms: tail_mod,
                hpf_hz: tail_hpf,
                lpf_hz: tail_lpf,
                sample_rate: self.sample_rate,
            };

            // Reconfigure reverb only when params actually change.
            let cache_key = TailParamsCache {
                time_ms: tail_time,
                feedback: tail_feedback,
                diffusion: tail_diffusion,
                modulation_ms: tail_mod,
                hpf_hz: tail_hpf,
                lpf_hz: tail_lpf,
            };
            if self.tail_cache.as_ref() != Some(&cache_key) {
                self.reverb.configure(&tp);
                self.tail_cache = Some(cache_key);
            }

            let mut iter = channel_samples.iter_mut();
            let in_l = *iter.next().unwrap();
            let in_r = iter.next().copied().unwrap_or(in_l);

            // Classic shimmer topology: Input → Reverb → Pitch → Feedback → Mix.
            let delay_in_l = in_l + self.fb_state_l;
            let delay_in_r = in_r + self.fb_state_r;

            let (raw_tail_l, raw_tail_r, early_l, early_r) =
                self.reverb.process(delay_in_l, delay_in_r, &tp);

            // Spread: blend between mono mix and full stereo FDN output.
            let tail_mono = (raw_tail_l + raw_tail_r) * 0.5;
            let tail_l = tail_mono + (raw_tail_l - tail_mono) * tone_spread;
            let tail_r = tail_mono + (raw_tail_r - tail_mono) * tone_spread;

            // Buffer the reverb tail for the granular pitch shifter.
            self.buffer_l.write(tail_l);
            self.buffer_r.write(tail_r);

            let shimmer_on = self.params.shimmer_enabled.value();
            if !shimmer_on && self.shimmer_was_on {
                self.pool_l.reset();
                self.pool_r.reset();
                self.grain_delay.reset();
                self.fb_state_l = 0.0;
                self.fb_state_r = 0.0;
            }
            self.shimmer_was_on = shimmer_on;

            let (pitched_l, pitched_r) = if shimmer_on {
                let pl = self.pool_l.process_sample(&gp, &self.buffer_l);
                let pr = self.pool_r.process_sample(&gp, &self.buffer_r);
                (pl, pr)
            } else {
                (0.0, 0.0)
            };

            // Grain delay applied to pitched shimmer signal.
            let (delayed_l, delayed_r) = if shimmer_on {
                self.grain_delay.process(
                    pitched_l,
                    pitched_r,
                    gd_time,
                    gd_feedback,
                    gd_mode,
                    self.sample_rate,
                    gd_mix,
                    gd_detune,
                    gd_hpf,
                    gd_lpf,
                    gd_duck,
                )
            } else {
                (0.0, 0.0)
            };

            // Regen: pitched output recirculated back into reverb input.
            if shimmer_on {
                self.fb_state_l = soft_clip(self.dc_l.process(delayed_l * regen));
                self.fb_state_r = soft_clip(self.dc_r.process(delayed_r * regen));
            }

            // Wet = early reflections + reverb tail + pitched+delayed shimmer branch.
            // tail_mix=0 → весь вклад в хвост, tail_mix=1 → весь вклад в shimmer.
            let tail_w = 0.70 * (1.0 - tail_mix);
            let shimmer_w = 0.70 * tail_mix;
            let wet_l = early_l * 0.30 + tail_l * tail_w + delayed_l * shimmer_w;
            let wet_r = early_r * 0.30 + tail_r * tail_w + delayed_r * shimmer_w;

            // Equal-power dry/wet crossfade.
            let dry_a = (dry_wet * 0.5 * std::f32::consts::PI).cos();
            let wet_a = (dry_wet * 0.5 * std::f32::consts::PI).sin();

            let mixed_l = in_l * dry_a + wet_l * wet_a;
            let mixed_r = in_r * dry_a + wet_r * wet_a;

            // M/S width on final output.
            let mid = (mixed_l + mixed_r) * 0.5;
            let side = (mixed_l - mixed_r) * 0.5 * output_width;
            let out_l = (mid + side) * out_gain;
            let out_r = (mid - side) * out_gain;

            let mut it = channel_samples.iter_mut();
            if let Some(s) = it.next() {
                *s = out_l;
            }
            if let Some(s) = it.next() {
                *s = out_r;
            }

            peak = peak.max(out_l.abs()).max(out_r.abs());
        }

        if self.params.editor_state.is_open() {
            let current = self.peak_meter.load(Ordering::Relaxed);
            let new = if peak > current {
                peak
            } else {
                current * self.peak_decay_weight
            };
            self.peak_meter.store(new, Ordering::Relaxed);
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for ShimmerGranular {
    const CLAP_ID: &'static str = "com.monoramens.shimmer-granular";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Granular shimmer reverb");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Stereo,
        ClapFeature::Reverb,
    ];
}

impl Vst3Plugin for ShimmerGranular {
    const VST3_CLASS_ID: [u8; 16] = *b"ShimmerGranular1";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Reverb];
}

nih_export_clap!(ShimmerGranular);
nih_export_vst3!(ShimmerGranular);

pub use ShimmerGranular as PluginType;
