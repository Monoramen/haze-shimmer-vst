/// Tiny realtime-safe PRNG (xorshift32). Not cryptographically secure.
/// RNG используется для джиттера: случайных отклонений позиции зерна в буфере и случайных отклонений высоты тона.
/// Важно, что он детерминированный, чтобы при одинаковом сидировании всегда выдавать одинаковую последовательность чисел.
pub struct Rng(u32);

impl Rng {
    pub fn new(seed: u32) -> Self {
        Self(if seed == 0 { 0x9E37_79B9 } else { seed })
    }

    #[inline]
    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }

    /// Uniform in [0, 1).
    #[inline]
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / ((1u32 << 24) as f32)
    }

    /// Uniform in [-1, 1).
    #[inline]
    pub fn next_bipolar(&mut self) -> f32 {
        self.next_f32() * 2.0 - 1.0
    }
}
