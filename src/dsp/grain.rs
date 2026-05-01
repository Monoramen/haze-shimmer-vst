use super::envelope::{self, EnvelopeShape};
use super::ring_buffer::RingBuffer;

#[derive(Clone, Copy)]
pub struct Grain {
    pub active: bool,
    pub current_delay: f64, // Текущая задержка от write head
    pub age: f64,
    pub length: f64,
    pub speed: f64,
    pub envelope: EnvelopeShape,
    pub gain: f32,
}

impl Grain {
    pub const INACTIVE: Self = Self {
        active: false,
        current_delay: 0.0,
        age: 0.0,
        length: 0.0,
        speed: 1.0,
        envelope: EnvelopeShape {
            attack: 0.5,
            hold: 0.0,
            attack_curve: 0.0,
            release_curve: 0.0, // или 1.0, как было в вашем примере
        },
        gain: 0.0,
    };
    // Для инициализации (вызывается в GrainPool::spawn)
    pub fn new(start_delay: f64, length: f64, speed: f64, env: EnvelopeShape, gain: f32) -> Self {
        Self {
            active: true,
            current_delay: start_delay,
            age: 0.0,
            length,
            speed,
            envelope: env,
            gain,
        }
    }

    #[inline]
    pub fn tick(&mut self, buffer: &RingBuffer) -> f32 {
        if !self.active {
            return 0.0;
        }

        if self.age >= self.length {
            self.active = false;
            return 0.0;
        }

        // Cubic interpolation needs 2 samples of lookahead past the read
        // position. If the grain has caught up to the write head (can happen
        // at speed < 1 after many ticks, or with aggressive position jitter),
        // bail out silently instead of reading unwritten data → click.
        if self.current_delay < 2.0 {
            self.active = false;
            return 0.0;
        }

        let sample = buffer.read_cubic(self.current_delay);

        // Применяем огибающую
        let phase = (self.age / self.length) as f32;
        let env = envelope::evaluate(self.envelope, phase);

        // Обновляем состояние:
        // При увеличении age, мы "догоняем" write head на значение speed
        self.current_delay -= self.speed;
        self.age += 1.0;

        sample * env * self.gain
    }
}
