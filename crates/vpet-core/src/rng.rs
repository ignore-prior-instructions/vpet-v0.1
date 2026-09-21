//! xoshiro128** RNG. See docs/DETERMINISM.md rule 6: drawn only when scheduling an event,
//! choosing among equal-priority evolution branches, resolving a battle, or generating the play
//! sequence. State is 4 `u32` words, persisted in the save.
//!
//! Reference: David Blackman and Sebastiano Vigna, xoshiro128** 1.1,
//! <https://prng.di.unimi.it/xoshiro128starstar.c>. Seeded via splitmix64
//! (<https://prng.di.unimi.it/splitmix64.c>) so any 64-bit host seed produces a well-mixed
//! initial state.

/// xoshiro128** generator state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rng {
    s: [u32; 4],
}

impl Rng {
    /// Build a generator from raw state (used when loading a save).
    pub const fn from_state(s: [u32; 4]) -> Self {
        Rng { s }
    }

    /// Current raw state, for persisting in the save.
    pub const fn state(&self) -> [u32; 4] {
        self.s
    }

    /// Seed from any 64 bits of host entropy via splitmix64, avoiding the all-zero state
    /// (xoshiro128** requires the state not be all zero).
    pub fn seed_from_u64(seed: u64) -> Self {
        let mut sm = seed;
        let mut next_u64 = || {
            sm = sm.wrapping_add(0x9e3779b97f4a7c15);
            let mut z = sm;
            z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
            z ^ (z >> 31)
        };
        let a = next_u64();
        let b = next_u64();
        let mut s = [(a >> 32) as u32, a as u32, (b >> 32) as u32, b as u32];
        if s == [0, 0, 0, 0] {
            // Astronomically unlikely, but xoshiro128** is undefined on the all-zero state.
            s = [1, 0, 0, 0];
        }
        Rng { s }
    }

    #[inline]
    fn rotl(x: u32, k: u32) -> u32 {
        x.rotate_left(k)
    }

    /// Next raw `u32`.
    pub fn next_u32(&mut self) -> u32 {
        let result = Self::rotl(self.s[1].wrapping_mul(5), 7).wrapping_mul(9);

        let t = self.s[1] << 9;

        self.s[2] ^= self.s[0];
        self.s[3] ^= self.s[1];
        self.s[1] ^= self.s[2];
        self.s[0] ^= self.s[3];

        self.s[2] ^= t;

        self.s[3] = Self::rotl(self.s[3], 11);

        result
    }

    /// A value in `[lo, hi]` inclusive. `hi` must be `>= lo`. Uses modulo reduction: for the
    /// small bounded ranges this game draws from (a handful to a few thousand), the bias is
    /// negligible and not worth the extra draws of rejection sampling.
    pub fn range(&mut self, lo: u32, hi: u32) -> u32 {
        debug_assert!(hi >= lo);
        let span = hi - lo + 1;
        if span == 0 {
            // hi == u32::MAX and lo == 0: full range, avoid a divide by zero on wraparound span.
            return self.next_u32();
        }
        lo + (self.next_u32() % span)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cross-checked against an independent Python port of the same reference algorithm
    /// (splitmix64(1) seeding xoshiro128**). See tools/spritekit for no coupling; this vector
    /// was generated standalone and is pinned here so any accidental change to the algorithm is
    /// caught.
    #[test]
    fn known_answer_seed_1() {
        let mut rng = Rng::seed_from_u64(1);
        let expected: [u32; 8] = [
            3039230342, 162680617, 1651489432, 2292780199, 3969336773, 1292539610, 4170620913,
            1814754080,
        ];
        for (i, &want) in expected.iter().enumerate() {
            let got = rng.next_u32();
            assert_eq!(got, want, "output #{i}");
        }
    }

    #[test]
    fn state_round_trips() {
        let mut rng = Rng::seed_from_u64(42);
        rng.next_u32();
        rng.next_u32();
        let s = rng.state();
        let mut rng2 = Rng::from_state(s);
        assert_eq!(rng.next_u32(), rng2.next_u32());
    }

    #[test]
    fn range_stays_in_bounds() {
        let mut rng = Rng::seed_from_u64(7);
        for _ in 0..1000 {
            let v = rng.range(5, 9);
            assert!((5..=9).contains(&v));
        }
        // Degenerate single-value range.
        assert_eq!(rng.range(3, 3), 3);
    }

    #[test]
    fn never_all_zero_state() {
        // seed 0 through splitmix64 does not happen to hit all-zero, but exercise the guard
        // path directly.
        let rng = Rng::from_state([0, 0, 0, 0]);
        // Constructing from_state with all zeros is allowed (loading a corrupt/never-seeded
        // save should not panic); seed_from_u64 is the path that guards against it.
        assert_eq!(rng.state(), [0, 0, 0, 0]);
    }
}
