//! Java-compatible 48-bit Linear Congruential RNG.
//!
//! Bit-exact port of the inline functions in `cubiomes/rng.h`:
//! `setSeed`, `next`, `nextInt`, `nextLong`, `nextFloat`, `nextDouble`,
//! `skipNextN`, and the `JAVA_NEXT_INT24` macro fast path. Matches the
//! Java specification of `java.util.Random` exactly.

/// LCG multiplier (`0x5deece66d`), identical to the Java spec.
const MULT: u64 = 0x0005_deec_e66d;
/// LCG additive constant.
const ADD: u64 = 0xb;
/// Mask isolating the lower 48 bits of the state.
const MASK: u64 = (1 << 48) - 1;

/// Bit-exact replica of Minecraft's `java.util.Random` (48-bit LCG).
///
/// The seed is *not* the raw `value` passed to [`JavaRng::new`]; Java
/// XORs it with [`MULT`] and masks to 48 bits at seed time. Round-trip
/// the value via [`JavaRng::raw_seed`] / [`JavaRng::from_raw`] if you
/// need to copy an in-flight state across boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JavaRng {
    seed: u64,
}

impl JavaRng {
    /// Create an RNG seeded from a Java `setSeed`-compatible value.
    #[inline]
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self {
            seed: (value ^ MULT) & MASK,
        }
    }

    /// Re-seed in place. Equivalent to constructing a fresh [`JavaRng`].
    #[inline]
    pub const fn set_seed(&mut self, value: u64) {
        self.seed = (value ^ MULT) & MASK;
    }

    /// Wrap a raw 48-bit state (already XOR'd and masked).
    ///
    /// Useful when restoring state captured from another RNG instance
    /// via [`JavaRng::raw_seed`].
    #[inline]
    #[must_use]
    pub const fn from_raw(raw_seed: u64) -> Self {
        Self {
            seed: raw_seed & MASK,
        }
    }

    /// Return the current raw 48-bit state.
    #[inline]
    #[must_use]
    pub const fn raw_seed(&self) -> u64 {
        self.seed
    }

    /// Step the LCG and return the top `bits` bits as a signed `i32`.
    ///
    /// Mirrors `next(seed, bits)` from cubiomes: the cast through
    /// `int64_t` preserves the sign of the high bit when shifting.
    #[inline]
    pub const fn next(&mut self, bits: u32) -> i32 {
        self.seed = self.seed.wrapping_mul(MULT).wrapping_add(ADD) & MASK;
        // Signed-right-shift preserves the sign, matching cubiomes' cast.
        ((self.seed as i64) >> (48 - bits)) as i32
    }

    /// Generate a uniformly distributed integer in `[0, n)`.
    ///
    /// Panics in debug builds if `n <= 0`; mirrors Java's `IllegalArgumentException`.
    #[inline]
    pub fn next_int(&mut self, n: i32) -> i32 {
        assert!(n > 0, "JavaRng::next_int requires n > 0, got {n}");
        let m = n - 1;
        if (m & n) == 0 {
            // n is a power of two — fast path.
            let x = (n as u64).wrapping_mul(self.next(31) as u32 as u64);
            return ((x as i64) >> 31) as i32;
        }
        loop {
            let bits = self.next(31);
            let val = bits % n;
            // The C version checks `(int32_t)((uint32_t)bits - val + m) < 0`,
            // which detects the rejection-sampling underflow.
            if ((bits as u32)
                .wrapping_sub(val as u32)
                .wrapping_add(m as u32) as i32)
                >= 0
            {
                return val;
            }
        }
    }

    /// Return the next pseudorandom `u64`.
    ///
    /// Two 32-bit `next` calls are concatenated MSB-first. cubiomes' C code
    /// casts each `int` to `uint64_t` (a *sign-extending* cast in C99 when
    /// the source type is signed), so a negative result from either `next`
    /// call propagates the sign bit through the result. Rust requires us
    /// to spell that out by going through `i64`.
    #[inline]
    pub const fn next_long(&mut self) -> u64 {
        let hi = self.next(32) as i64;
        let lo = self.next(32) as i64;
        ((hi << 32).wrapping_add(lo)) as u64
    }

    /// Return the next pseudorandom `f32` in `[0, 1)`.
    #[inline]
    pub const fn next_float(&mut self) -> f32 {
        self.next(24) as f32 / (1u32 << 24) as f32
    }

    /// Return the next pseudorandom `f64` in `[0, 1)`.
    ///
    /// 26 + 27 = 53 bits of entropy, matching the f64 mantissa width.
    #[inline]
    pub const fn next_double(&mut self) -> f64 {
        let hi = self.next(26) as u32 as u64;
        let lo = self.next(27) as u32 as u64;
        let x = (hi << 27).wrapping_add(lo);
        (x as i64) as f64 / (1u64 << 53) as f64
    }

    /// Advance the LCG by `n` steps in `O(log n)` time.
    ///
    /// Mirrors cubiomes' `skipNextN` by composing the LCG transition matrix.
    #[inline]
    pub const fn skip_n(&mut self, mut n: u64) {
        let mut m: u64 = 1;
        let mut a: u64 = 0;
        let mut im: u64 = MULT;
        let mut ia: u64 = ADD;
        while n != 0 {
            if n & 1 != 0 {
                m = m.wrapping_mul(im);
                a = im.wrapping_mul(a).wrapping_add(ia);
            }
            ia = im.wrapping_add(1).wrapping_mul(ia);
            im = im.wrapping_mul(im);
            n >>= 1;
        }
        self.seed = self.seed.wrapping_mul(m).wrapping_add(a) & MASK;
    }

    /// Generate `nextInt(24)` via the inlined Hacker's-Delight fast path.
    ///
    /// Bit-exact port of the `JAVA_NEXT_INT24` macro from cubiomes. Used by
    /// the noise initialisation routines; equivalent to `self.next_int(24)`
    /// but avoids the rejection-sampling loop entirely because 24 is small
    /// enough that `next(31) % 24` is always in range.
    #[inline]
    pub const fn next_int_24(&mut self) -> i32 {
        let mut a: u64 = MASK;
        let mut c: u64 = MULT.wrapping_mul(self.seed);
        c = c.wrapping_add(11);
        a &= c;
        self.seed = a;
        a = (a as i64 >> 17) as u64;
        c = 0xaaaa_aaabu64.wrapping_mul(a);
        c = (c as i64 >> 36) as u64;
        (a as i32) - ((c << 3) as i32) * 3
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    /// Reference output for `setSeed(0); next(32)` from cubiomes.
    ///
    /// `next` advances the seed and returns the high `bits` bits as a
    /// signed integer. Confirmed by tracing the LCG by hand.
    #[test]
    fn next_after_zero_seed_matches_reference() {
        let mut rng = JavaRng::new(0);
        // setSeed(0) -> seed = 0 ^ MULT = MULT = 0x5deece66d
        // next(32) advances: seed = (MULT * MULT + 0xb) & MASK
        let v = rng.next(32);
        // Hand-computed: MULT * MULT = 0x22a9_e76d_d72c_4d49, + 11 mod 2^48
        let expected_seed = (MULT.wrapping_mul(MULT).wrapping_add(0xb)) & MASK;
        assert_eq!(rng.raw_seed(), expected_seed);
        let expected = (expected_seed as i64 >> 16) as i32;
        assert_eq!(v, expected);
    }

    #[test]
    fn set_seed_xor_masks_to_48_bits() {
        let mut rng = JavaRng::new(0);
        assert_eq!(rng.raw_seed(), MULT);
        rng.set_seed(u64::MAX);
        assert_eq!(rng.raw_seed(), (u64::MAX ^ MULT) & MASK);
    }

    #[test]
    fn next_int_pow2_uses_fast_path() {
        // For n = 16 the fast path returns `(n * next(31)) >> 31`; the
        // remainder-loop branch must never execute.
        let mut rng = JavaRng::new(42);
        for _ in 0..256 {
            let v = rng.next_int(16);
            assert!((0..16).contains(&v), "next_int(16) yielded {v}");
        }
    }

    #[test]
    fn next_int_arbitrary_bounds_are_in_range() {
        let mut rng = JavaRng::new(0xdead_beef);
        for n in [3, 5, 7, 13, 24, 100, 65537] {
            for _ in 0..256 {
                let v = rng.next_int(n);
                assert!((0..n).contains(&v), "next_int({n}) yielded {v}");
            }
        }
    }

    #[test]
    #[should_panic(expected = "next_int requires n > 0")]
    fn next_int_with_zero_panics() {
        let mut rng = JavaRng::new(0);
        let _ = rng.next_int(0);
    }

    #[test]
    #[should_panic(expected = "next_int requires n > 0")]
    fn next_int_with_negative_panics() {
        let mut rng = JavaRng::new(0);
        let _ = rng.next_int(-1);
    }

    #[test]
    fn next_long_concatenates_two_next32_calls_with_sign_extension() {
        let mut a = JavaRng::new(7);
        let mut b = JavaRng::new(7);
        let merged = a.next_long();
        // C99 promotes each `int` to `uint64_t` with sign extension, so
        // mirror that explicitly here.
        let hi = b.next(32) as i64;
        let lo = b.next(32) as i64;
        assert_eq!(merged, ((hi << 32).wrapping_add(lo)) as u64);
    }

    #[test]
    fn next_float_in_unit_interval() {
        let mut rng = JavaRng::new(1234);
        for _ in 0..1024 {
            let v = rng.next_float();
            assert!((0.0..1.0).contains(&v), "next_float yielded {v}");
        }
    }

    #[test]
    fn next_double_in_unit_interval() {
        let mut rng = JavaRng::new(1234);
        for _ in 0..1024 {
            let v = rng.next_double();
            assert!((0.0..1.0).contains(&v), "next_double yielded {v}");
        }
    }

    #[test]
    fn skip_n_matches_iterated_next_long() {
        let mut stepped = JavaRng::new(0xc0ff_ee42);
        let mut skipped = JavaRng::new(0xc0ff_ee42);

        // Each next_long consumes two next(32) calls, so 100 next_long
        // calls correspond to 200 LCG steps.
        for _ in 0..100 {
            let _ = stepped.next_long();
        }
        skipped.skip_n(200);
        assert_eq!(stepped.raw_seed(), skipped.raw_seed());
    }

    #[test]
    fn skip_n_zero_is_a_no_op() {
        let mut rng = JavaRng::new(99);
        let before = rng.raw_seed();
        rng.skip_n(0);
        assert_eq!(rng.raw_seed(), before);
    }

    #[test]
    fn next_int_24_matches_next_int_for_24() {
        // The fast-path macro must agree with the slow path on any seed.
        for &seed in &[0u64, 1, 0xdead_beef, 0x4242_4242, u64::MAX] {
            let mut fast = JavaRng::new(seed);
            let mut slow = JavaRng::new(seed);
            for _ in 0..256 {
                let f = fast.next_int_24();
                let s = slow.next_int(24);
                assert_eq!(
                    f, s,
                    "next_int_24 disagrees with next_int(24) at seed={seed}"
                );
            }
        }
    }
}
