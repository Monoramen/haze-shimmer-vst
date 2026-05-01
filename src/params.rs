use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use std::sync::Arc;

use crate::editor;

#[derive(Params)]
pub struct ShimmerParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,

    #[id = "grain_size_ms"]
    pub grain_size_ms: FloatParam,

    #[id = "pitch_shift_semitones"]
    pub pitch_shift_semitones: FloatParam,

    #[id = "grain_density"]
    pub grain_density: FloatParam,

    /// Attack as fraction of grain length (0 = instant, 1 = whole grain ramps in).
    #[id = "env_attack"]
    pub env_attack: FloatParam,

    /// Sustain/hold at peak, as fraction of grain length.
    #[id = "env_hold"]
    pub env_hold: FloatParam,

    /// Curve shape. −1 = logarithmic (fast rise / slow fall), 0 = linear, +1 = exponential.
    #[id = "env_attack_curve"]
    pub env_attack_curve: FloatParam,

    #[id = "env_release_curve"]
    pub env_release_curve: FloatParam,

    #[id = "position_jitter"]
    pub position_jitter: FloatParam,

    #[id = "pitch_jitter"]
    pub pitch_jitter: FloatParam,

    #[id = "shimmer_enabled"]
    pub shimmer_enabled: BoolParam,

    #[id = "grain_delay_time_ms"]
    pub grain_delay_time_ms: FloatParam,

    #[id = "grain_delay_feedback"]
    pub grain_delay_feedback: FloatParam,

    #[id = "grain_delay_mode"]
    pub grain_delay_mode: IntParam,

    #[id = "grain_delay_sync"]
    pub grain_delay_sync: BoolParam,

    /// Note division index: 0=1/1, 1=1/2, 2=1/4, 3=1/8, 4=1/16, 5=1/4T, 6=1/8T
    #[id = "grain_delay_division"]
    pub grain_delay_division: IntParam,

    #[id = "tail_time_ms"]
    pub tail_time_ms: FloatParam,

    #[id = "tail_feedback"]
    pub tail_feedback: FloatParam,

    #[id = "tail_diffusion"]
    pub tail_diffusion: FloatParam,

    #[id = "tail_modulation_ms"]
    pub tail_modulation_ms: FloatParam,

    #[id = "tail_hpf"]
    pub tail_hpf: FloatParam,

    #[id = "tail_lpf"]
    pub tail_lpf: FloatParam,

    #[id = "regen"]
    pub regen: FloatParam,

    #[id = "dry_wet"]
    pub dry_wet: FloatParam,

    #[id = "output_gain"]
    pub output_gain: FloatParam,

    /// M/S stereo width on final output. 0=mono, 1=unchanged, 2=extra wide.
    #[id = "output_width"]
    pub output_width: FloatParam,

    /// FDN stereo spread: blends L/R channels of FDN output (0=mono mix, 1=full stereo).
    #[id = "tone_spread"]
    pub tone_spread: FloatParam,

    /// Grain delay dry/wet mix (0=dry, 1=full wet).
    #[id = "gd_mix"]
    pub gd_mix: FloatParam,

    /// Grain delay stereo pan offset in samples (L early → R late).
    #[id = "gd_pan"]
    pub gd_pan: FloatParam,

    /// Grain delay inter-channel detune in cents.
    #[id = "gd_detune"]
    pub gd_detune: FloatParam,

    /// Grain delay HPF cutoff Hz.
    #[id = "gd_hpf"]
    pub gd_hpf: FloatParam,

    /// Grain delay LPF cutoff Hz.
    #[id = "gd_lpf"]
    pub gd_lpf: FloatParam,

    /// Grain delay duck amount: how much input signal suppresses delay output (0=off, 1=full duck).
    #[id = "gd_duck"]
    pub gd_duck: FloatParam,

    /// Balance between reverb tail and shimmer (pitched grains). 0=all tail, 1=all shimmer.
    #[id = "tail_mix"]
    pub tail_mix: FloatParam,
}

impl Default for ShimmerParams {
    fn default() -> Self {
        Self {
            editor_state: editor::default_state(),

            grain_size_ms: FloatParam::new(
                "Grain Size",
                60.0,
                FloatRange::Skewed {
                    min: 10.0,
                    max: 400.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_unit(" ms")
            .with_smoother(SmoothingStyle::Linear(30.0))
            .with_value_to_string(Arc::new(|v| format!("{v:.1}"))),

            pitch_shift_semitones: FloatParam::new(
                "Pitch Shift",
                12.0,
                FloatRange::Linear {
                    min: -36.0,
                    max: 36.0,
                },
            )
            .with_unit(" st")
            .with_smoother(SmoothingStyle::Linear(50.0))
            .with_value_to_string(Arc::new(|v| format!("{v:+.1}"))),

            grain_density: FloatParam::new(
                "Density",
                20.0,
                FloatRange::Skewed {
                    min: 10.0,
                    max: 100.0,
                    factor: FloatRange::skew_factor(-0.5),
                },
            )
            .with_unit(" g/s")
            .with_smoother(SmoothingStyle::Linear(30.0))
            .with_value_to_string(Arc::new(|v| format!("{v:.1}"))),

            env_attack: FloatParam::new("Attack", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),

            env_hold: FloatParam::new("Hold", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),

            env_attack_curve: FloatParam::new(
                "Attack Curve",
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            )
            .with_value_to_string(Arc::new(|v| {
                if v.abs() < 0.02 {
                    "Lin".to_string()
                } else if v < 0.0 {
                    format!("Log {v:+.2}")
                } else {
                    format!("Exp {v:+.2}")
                }
            })),

            // В ShimmerParams
            env_release_curve: FloatParam::new(
                "Release Curve",
                1.0, // По умолчанию Exp (плавное затухание)
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                }, // Ставим от -1 до 1
            )
            .with_value_to_string(Arc::new(|v| {
                if v.abs() < 0.02 {
                    "Lin".into()
                } else if v < 0.0 {
                    format!("Log {v:+.2}")
                } else {
                    format!("Exp {v:+.2}")
                }
            })),
            position_jitter: FloatParam::new(
                "Pos Jitter",
                0.2,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_string_to_value(formatters::s2v_f32_percentage()),

            pitch_jitter: FloatParam::new(
                "Pitch Drift",
                0.0,
                FloatRange::Skewed {
                    min: 0.0,
                    max: 400.0,
                    factor: FloatRange::skew_factor(-1.5),
                },
            )
            .with_unit(" ct")
            .with_value_to_string(Arc::new(|v| format!("{v:.1}"))),

            shimmer_enabled: BoolParam::new("Shimmer", true),

            grain_delay_time_ms: FloatParam::new(
                "Grain Delay",
                250.0,
                FloatRange::Skewed {
                    min: 10.0,
                    max: 2000.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_unit(" ms")
            .with_smoother(SmoothingStyle::Linear(50.0))
            .with_value_to_string(Arc::new(|v| format!("{v:.0}"))),

            grain_delay_feedback: FloatParam::new(
                "Delay FB",
                0.4,
                FloatRange::Linear {
                    min: 0.0,
                    max: 0.95,
                },
            )
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_string_to_value(formatters::s2v_f32_percentage())
            .with_smoother(SmoothingStyle::Linear(30.0)),

            grain_delay_mode: IntParam::new("Delay Mode", 0, IntRange::Linear { min: 0, max: 2 }),

            grain_delay_sync: BoolParam::new("Delay Sync", false),

            grain_delay_division: IntParam::new(
                "Delay Division",
                6, // default 1/4
                IntRange::Linear { min: 0, max: 13 },
            )
            .with_value_to_string(Arc::new(|v| match v as i32 {
                0 => "4/1".to_string(),
                1 => "3/1".to_string(),
                2 => "2/1".to_string(),
                3 => "1/1".to_string(),
                4 => "1/2".to_string(),
                5 => "1/2T".to_string(),
                6 => "1/4".to_string(),
                7 => "1/4T".to_string(),
                8 => "1/8".to_string(),
                9 => "1/8T".to_string(),
                10 => "1/16".to_string(),
                11 => "1/16T".to_string(),
                12 => "1/32".to_string(),
                _ => "1/32T".to_string(),
            })),

            tail_time_ms: FloatParam::new(
                "Tail Time",
                480.0,
                FloatRange::Skewed {
                    min: 50.0,
                    max: 6000.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_unit(" ms")
            .with_smoother(SmoothingStyle::Linear(80.0))
            .with_value_to_string(Arc::new(|v| format!("{v:.0}"))),

            tail_feedback: FloatParam::new(
                "Tail Feedback",
                0.55,
                FloatRange::Linear { min: 0.0, max: 1.1 },
            )
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_string_to_value(formatters::s2v_f32_percentage())
            .with_smoother(SmoothingStyle::Linear(30.0)),

            tail_diffusion: FloatParam::new(
                "Diffusion",
                0.6,
                FloatRange::Linear { min: 0.0, max: 1.0 },
            )
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_string_to_value(formatters::s2v_f32_percentage())
            .with_smoother(SmoothingStyle::Linear(30.0)),

            tail_modulation_ms: FloatParam::new(
                "Modulation",
                2.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 10.0,
                },
            )
            .with_unit(" ms")
            .with_smoother(SmoothingStyle::Linear(30.0))
            .with_value_to_string(Arc::new(|v| format!("{v:.2}"))),

            tail_hpf: FloatParam::new(
                "Tail HPF",
                120.0,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 500.0,
                    factor: FloatRange::skew_factor(-1.0),
                },
            )
            .with_unit(" Hz")
            .with_smoother(SmoothingStyle::Logarithmic(30.0))
            .with_value_to_string(Arc::new(|v| format!("{v:.0}"))),

            tail_lpf: FloatParam::new(
                "Tail LPF",
                6000.0,
                FloatRange::Skewed {
                    min: 1000.0,
                    max: 20000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_unit(" Hz")
            .with_smoother(SmoothingStyle::Logarithmic(30.0))
            .with_value_to_string(Arc::new(|v| format!("{v:.0}"))),

            regen: FloatParam::new(
                "Regen",
                0.35,
                FloatRange::Linear {
                    min: 0.0,
                    max: 0.95,
                },
            )
            .with_unit(" %")
            .with_value_to_string(formatters::v2s_f32_percentage(0))
            .with_string_to_value(formatters::s2v_f32_percentage())
            .with_smoother(SmoothingStyle::Linear(30.0)),

            dry_wet: FloatParam::new("Dry/Wet", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit(" %")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage())
                .with_smoother(SmoothingStyle::Linear(30.0)),

            output_gain: FloatParam::new(
                "Output",
                util::db_to_gain(0.0),
                FloatRange::Skewed {
                    min: util::db_to_gain(-24.0),
                    max: util::db_to_gain(12.0),
                    factor: FloatRange::gain_skew_factor(-24.0, 12.0),
                },
            )
            .with_unit(" dB")
            .with_smoother(SmoothingStyle::Logarithmic(30.0))
            .with_value_to_string(formatters::v2s_f32_gain_to_db(1))
            .with_string_to_value(formatters::s2v_f32_gain_to_db()),

            output_width: FloatParam::new("Width", 1.0, FloatRange::Linear { min: 0.0, max: 2.0 })
                .with_smoother(SmoothingStyle::Linear(30.0))
                .with_value_to_string(Arc::new(|v| format!("{:.0}%", v * 100.0))),

            tone_spread: FloatParam::new("Spread", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit(" %")
                .with_smoother(SmoothingStyle::Linear(30.0))
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),

            gd_mix: FloatParam::new("GD Mix", 1.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit(" %")
                .with_smoother(SmoothingStyle::Linear(30.0))
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),

            gd_pan: FloatParam::new(
                "Pan",
                0.0,
                FloatRange::Linear {
                    min: -1.0,
                    max: 1.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(30.0))
            .with_value_to_string(Arc::new(|v| {
                if v.abs() < 0.02 {
                    "C".to_string()
                } else if v < 0.0 {
                    format!("L{:.0}", v.abs() * 100.0)
                } else {
                    format!("R{:.0}", v * 100.0)
                }
            })),

            gd_detune: FloatParam::new(
                "Detune",
                0.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 50.0,
                },
            )
            .with_unit(" ct")
            .with_smoother(SmoothingStyle::Linear(30.0))
            .with_value_to_string(Arc::new(|v| format!("{v:.1}"))),

            gd_hpf: FloatParam::new(
                "GD HPF",
                20.0,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 2000.0,
                    factor: FloatRange::skew_factor(-1.5),
                },
            )
            .with_unit(" Hz")
            .with_smoother(SmoothingStyle::Logarithmic(30.0))
            .with_value_to_string(Arc::new(|v| format!("{v:.0}"))),

            gd_lpf: FloatParam::new(
                "GD LPF",
                20000.0,
                FloatRange::Skewed {
                    min: 500.0,
                    max: 20000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_unit(" Hz")
            .with_smoother(SmoothingStyle::Logarithmic(30.0))
            .with_value_to_string(Arc::new(|v| format!("{v:.0}"))),

            gd_duck: FloatParam::new("Duck", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit(" %")
                .with_smoother(SmoothingStyle::Linear(30.0))
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),

            tail_mix: FloatParam::new("Tail Mix", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_unit(" %")
                .with_smoother(SmoothingStyle::Linear(30.0))
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),
        }
    }
}
