/// Parametric attack-hold-release window with curve shaping.
///
/// `attack` and `hold` are fractions of the grain length; `release` is the
/// remaining portion (`1 - attack - hold`, clamped). If attack+hold overflow,
/// they are renormalised. `curve` in [-1, +1]: 0 = linear, +1 = exponential
/// (slow rise / fast fall), -1 = logarithmic (fast rise / slow fall).
#[derive(Clone, Copy, Debug)]
pub struct EnvelopeShape {
    pub attack: f32,
    pub hold: f32,
    pub attack_curve: f32,
    pub release_curve: f32,
}

impl Default for EnvelopeShape {
    fn default() -> Self {
        // Hann-like bell (A=0.5, H=0, linear) is a decent start.
        Self {
            attack: 0.5,
            hold: 0.0,
            attack_curve: 0.0,
            release_curve: 1.0,
        }
    }
}

#[inline]
pub fn evaluate(shape: EnvelopeShape, phase: f32) -> f32 {
    let p = phase.clamp(0.0, 1.0);

    // Minimum release as a fraction of the grain. Even when the user picks
    // attack+hold ≈ 1.0, we force a short ramp to zero at the tail so grains
    // never terminate on a non-zero sample (audible click).
    const MIN_RELEASE: f32 = 0.02;

    let mut a = shape.attack.clamp(0.0, 1.0);
    let mut h = shape.hold.clamp(0.0, 1.0);

    if a + h > 1.0 - MIN_RELEASE {
        let sum = (a + h) / (1.0 - MIN_RELEASE);
        a /= sum;
        h /= sum;
    }
    let r = (1.0 - a - h).max(MIN_RELEASE);

    // Обработка фаз
    if p <= a {
        if a <= f32::EPSILON {
            return 1.0;
        }
        // Используем attack_curve
        shape_ramp(p / a, shape.attack_curve)
    } else if p <= a + h {
        1.0
    } else if r > f32::EPSILON {
        let t = (p - a - h) / r;
        // Используем release_curve
        shape_ramp(1.0 - t, shape.release_curve)
    } else {
        0.0
    }
}

/// Map [0,1] through a curve: 0 = linear, +1 = exponential (slow-in / fast-out),
/// -1 = logarithmic (fast-in / slow-out). t=0→0, t=1→1 always.
#[inline]
fn shape_ramp(t: f32, curve: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if curve.abs() < 1e-4 {
        return t;
    }
    // Exponent in [1/4, 4] via 4^curve — smooth monotonic around 1.
    let k = 4.0f32.powf(curve);
    t.powf(k)
}
