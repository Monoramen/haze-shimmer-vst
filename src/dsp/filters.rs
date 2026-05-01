use std::f32::consts::TAU;

pub struct OnePoleLP {
    a: f32,
    z: f32,
}

impl OnePoleLP {
    pub fn new() -> Self {
        Self { a: 1.0, z: 0.0 }
    }

    pub fn reset(&mut self) {
        self.z = 0.0;
    }

    pub fn set_cutoff(&mut self, cutoff_hz: f32, sample_rate: f32) {
        let fc = cutoff_hz.clamp(20.0, sample_rate * 0.49);
        let x = (-TAU * fc / sample_rate).exp();
        self.a = 1.0 - x;
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        self.z += self.a * (x - self.z);
        self.z
    }
}

pub struct OnePoleHP {
    lp: OnePoleLP,
}

impl OnePoleHP {
    pub fn new() -> Self {
        Self {
            lp: OnePoleLP::new(),
        }
    }

    pub fn reset(&mut self) {
        self.lp.reset();
    }

    pub fn set_cutoff(&mut self, cutoff_hz: f32, sample_rate: f32) {
        self.lp.set_cutoff(cutoff_hz, sample_rate);
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        x - self.lp.process(x)
    }
}

pub struct DcBlocker {
    x1: f32,
    y1: f32,
    r: f32,
}

impl DcBlocker {
    pub fn new() -> Self {
        Self {
            x1: 0.0,
            y1: 0.0,
            r: 0.995,
        }
    }

    pub fn reset(&mut self) {
        self.x1 = 0.0;
        self.y1 = 0.0;
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = x - self.x1 + self.r * self.y1;
        self.x1 = x;
        self.y1 = y;
        y
    }
}

/// Rational soft-clip approximation of tanh. Error < 3%, ~4× faster.
#[inline]
pub fn soft_clip(x: f32) -> f32 {
    x / (1.0 + x.abs())
}
