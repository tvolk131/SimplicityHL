//! A tiny deterministic PRNG.
//!
//! The corpus must be bit-for-bit reproducible across machines and crate
//! versions, so it cannot depend on `rand` (whose streams are not guaranteed
//! stable). xorshift64* has plenty of quality for choosing integer literals.

/// Seeded xorshift64* generator.
#[derive(Debug, Clone)]
pub struct Rng {
    state: u64,
}

impl Rng {
    /// Create a generator from `seed`. Seeds are hashed so that similar seeds
    /// still produce unrelated streams.
    pub fn new(seed: u64) -> Self {
        // xorshift64* degenerates on a zero state; the multiplier avoids it.
        Self {
            state: seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1,
        }
    }

    /// Next raw value.
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A value in `[lo, hi]` (inclusive). Only used for literals, where
    /// near-uniformity is good enough.
    pub fn u32_in(&mut self, lo: u32, hi: u32) -> u32 {
        let span = u64::from(hi - lo + 1);
        lo + (self.next_u64() % span) as u32
    }

    /// A small decimal literal, kept readable in generated sources.
    pub fn literal(&mut self) -> u32 {
        self.u32_in(1, 999_999)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_in_range() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
        assert_eq!(Rng::new(0).next_u64(), Rng::new(0).next_u64());

        let mut rng = Rng::new(7);
        assert_eq!(rng.u32_in(5, 5), 5);
        for _ in 0..100 {
            let v = rng.u32_in(3, 9);
            assert!((3..=9).contains(&v));
        }
    }
}
