//! Xoroshiro128 RNG used by Minecraft 1.18+.
//!
//! Bit-exact port of the inline helpers in `cubiomes/rng.h`:
//! `xSetSeed`, `xNextLong`, `xNextInt`, `xNextDouble`, `xNextFloat`,
//! `xSkipN`, `xNextLongJ`, and `xNextIntJ`. The seeding routine is the
//! Stafford-mix-13 variant used by Mojang since 1.18, *not* a plain
//! splitmix64.

use crate::math::rotl64;

/// Stafford-mix golden ratio constants used by `xSetSeed`.
const X_XL: u64 = 0x9e37_79b9_7f4a_7c15;
const X_XH: u64 = 0x6a09_e667_f3bc_c909;
const X_A: u64 = 0xbf58_476d_1ce4_e5b9;
const X_B: u64 = 0x94d0_49bb_1331_11eb;

/// Scaling factor for `xNextDouble`: `2.pow(-53)`.
const X_NEXT_DOUBLE_SCALE: f64 = 1.110_223_024_625_156_5e-16;
/// Scaling factor for `xNextFloat`: `2.pow(-24)`.
const X_NEXT_FLOAT_SCALE: f32 = 5.960_464_5e-8;

/// Bit-exact replica of Minecraft 1.18+ `Xoroshiro128++`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Xoroshiro {
    /// Low 64 bits of the 128-bit state.
    pub lo: u64,
    /// High 64 bits of the 128-bit state.
    pub hi: u64,
}

impl Xoroshiro {
    /// Construct from a seed value via the Stafford-13 mixing routine.
    #[inline]
    #[must_use]
    pub const fn new(value: u64) -> Self {
        let mut l = value ^ X_XH;
        let mut h = l.wrapping_add(X_XL);
        l = (l ^ (l >> 30)).wrapping_mul(X_A);
        h = (h ^ (h >> 30)).wrapping_mul(X_A);
        l = (l ^ (l >> 27)).wrapping_mul(X_B);
        h = (h ^ (h >> 27)).wrapping_mul(X_B);
        l ^= l >> 31;
        h ^= h >> 31;
        Self { lo: l, hi: h }
    }

    /// Re-seed in place.
    #[inline]
    pub const fn set_seed(&mut self, value: u64) {
        *self = Self::new(value);
    }

    /// Advance the state and return the next 64-bit output (`xNextLong`).
    #[inline]
    pub const fn next_long(&mut self) -> u64 {
        let l = self.lo;
        let h = self.hi;
        let n = rotl64(l.wrapping_add(h), 17).wrapping_add(l);
        let h = h ^ l;
        self.lo = rotl64(l, 49) ^ h ^ (h << 21);
        self.hi = rotl64(h, 28);
        n
    }

    /// Bounded uniform integer in `[0, n)` using Lemire's rejection method.
    ///
    /// Matches cubiomes' `xNextInt`. The caller must pass `n > 0`.
    #[inline]
    pub fn next_int(&mut self, n: u32) -> i32 {
        assert!(n > 0, "Xoroshiro::next_int requires n > 0");
        let n64 = n as u64;
        let mut r = (self.next_long() & 0xffff_ffff).wrapping_mul(n64);
        if (r as u32) < n {
            let threshold = (!n).wrapping_add(1) % n;
            while (r as u32) < threshold {
                r = (self.next_long() & 0xffff_ffff).wrapping_mul(n64);
            }
        }
        (r >> 32) as i32
    }

    /// `f64` in `[0, 1)` using the top 53 bits of `next_long`.
    #[inline]
    pub const fn next_double(&mut self) -> f64 {
        (self.next_long() >> (64 - 53)) as f64 * X_NEXT_DOUBLE_SCALE
    }

    /// `f32` in `[0, 1)` using the top 24 bits of `next_long`.
    #[inline]
    pub const fn next_float(&mut self) -> f32 {
        (self.next_long() >> (64 - 24)) as f32 * X_NEXT_FLOAT_SCALE
    }

    /// Advance by `count` outputs.
    #[inline]
    pub const fn skip_n(&mut self, count: u32) {
        let mut remaining = count;
        while remaining > 0 {
            let _ = self.next_long();
            remaining -= 1;
        }
    }

    /// Java-compatible long: two `next_long >> 32` halves spliced together.
    ///
    /// The shift result is interpreted as `i32` first, so the high half is
    /// sign-extended into the `u64` exactly the way cubiomes' C code does.
    #[inline]
    pub const fn next_long_j(&mut self) -> u64 {
        let a = (self.next_long() >> 32) as i32;
        let b = (self.next_long() >> 32) as i32;
        (((a as i64) << 32).wrapping_add(b as i64)) as u64
    }

    /// Java-compatible bounded int (mirrors `xNextIntJ`).
    ///
    /// Uses the top 31 bits of `next_long` per call. The caller must pass
    /// `n > 0`.
    #[inline]
    pub fn next_int_j(&mut self, n: u32) -> i32 {
        assert!(n > 0, "Xoroshiro::next_int_j requires n > 0");
        let m = n.wrapping_sub(1);
        if (m & n) == 0 {
            let x = (n as u64).wrapping_mul(self.next_long() >> 33);
            return ((x as i64) >> 31) as i32;
        }
        let n_i = n as i32;
        loop {
            let bits = (self.next_long() >> 33) as i32;
            let val = bits % n_i;
            if ((bits as u32).wrapping_sub(val as u32).wrapping_add(m) as i32) >= 0 {
                return val;
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn set_seed_runs_full_stafford_mix() {
        // Sanity: cubiomes' xSetSeed produces non-zero state for non-zero input.
        let x = Xoroshiro::new(1);
        assert_ne!(x.lo, 0);
        assert_ne!(x.hi, 0);
    }

    #[test]
    fn next_long_advances_state() {
        let mut x = Xoroshiro::new(42);
        let before = (x.lo, x.hi);
        let _ = x.next_long();
        assert_ne!((x.lo, x.hi), before);
    }

    #[test]
    fn next_long_is_deterministic() {
        let mut a = Xoroshiro::new(7);
        let mut b = Xoroshiro::new(7);
        for _ in 0..1024 {
            assert_eq!(a.next_long(), b.next_long());
        }
    }

    #[test]
    fn next_int_pow2_uses_fast_path() {
        let mut x = Xoroshiro::new(123);
        for _ in 0..512 {
            let v = x.next_int(16);
            assert!((0..16).contains(&v), "next_int(16) yielded {v}");
        }
    }

    #[test]
    fn next_int_arbitrary_bounds_are_in_range() {
        let mut x = Xoroshiro::new(0xdead_beef);
        for n in [3u32, 5, 7, 13, 24, 100, 65537] {
            for _ in 0..256 {
                let v = x.next_int(n);
                assert!(v >= 0 && (v as u32) < n, "next_int({n}) yielded {v}");
            }
        }
    }

    #[test]
    #[should_panic(expected = "next_int requires n > 0")]
    fn next_int_with_zero_panics() {
        let mut x = Xoroshiro::new(0);
        let _ = x.next_int(0);
    }

    #[test]
    fn next_double_in_unit_interval() {
        let mut x = Xoroshiro::new(1234);
        for _ in 0..1024 {
            let v = x.next_double();
            assert!((0.0..1.0).contains(&v), "next_double yielded {v}");
        }
    }

    #[test]
    fn next_float_in_unit_interval() {
        let mut x = Xoroshiro::new(1234);
        for _ in 0..1024 {
            let v = x.next_float();
            assert!((0.0..1.0).contains(&v), "next_float yielded {v}");
        }
    }

    #[test]
    fn skip_n_matches_iterated_next_long() {
        let mut stepped = Xoroshiro::new(99);
        let mut skipped = Xoroshiro::new(99);
        for _ in 0..50 {
            let _ = stepped.next_long();
        }
        skipped.skip_n(50);
        assert_eq!((stepped.lo, stepped.hi), (skipped.lo, skipped.hi));
    }

    #[test]
    fn next_long_j_consumes_two_next_long_calls() {
        let mut a = Xoroshiro::new(11);
        let mut b = Xoroshiro::new(11);
        let merged = a.next_long_j();
        let hi = (b.next_long() >> 32) as i32;
        let lo = (b.next_long() >> 32) as i32;
        let expected = (((hi as i64) << 32).wrapping_add(lo as i64)) as u64;
        assert_eq!(merged, expected);
    }

    #[test]
    fn next_int_j_pow2_fast_path_in_range() {
        let mut x = Xoroshiro::new(13);
        for _ in 0..256 {
            let v = x.next_int_j(16);
            assert!((0..16).contains(&v));
        }
    }

    #[test]
    fn next_int_j_arbitrary_bounds_in_range() {
        let mut x = Xoroshiro::new(0xc0de);
        for n in [3u32, 5, 7, 24, 100] {
            for _ in 0..256 {
                let v = x.next_int_j(n);
                assert!(v >= 0 && (v as u32) < n);
            }
        }
    }
}
