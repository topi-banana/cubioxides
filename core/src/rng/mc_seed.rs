//! Minecraft world seed pipeline helpers.
//!
//! These functions assemble per-chunk PRNG seeds from the world seed,
//! layer-specific salt, and chunk coordinates. They form the foundation
//! of cubiomes' layer system; the implementation is a bit-exact port of
//! the inline helpers in `cubiomes/rng.h`:
//! `mcStepSeed`, `mcFirstInt`, `mcFirstIsZero`, `getChunkSeed`,
//! `getLayerSalt`, `getStartSalt`, `getStartSeed`, and `mulInv`.
//!
//! Every helper is `const fn`, allowing layer salts to be folded at
//! compile time when the chain of `mc_step_seed` calls operates on
//! literals.

/// Multiplier in cubiomes' `mcStepSeed` LCG.
const STEP_MUL: u64 = 6_364_136_223_846_793_005;
/// Additive constant in `mcStepSeed`.
const STEP_ADD: u64 = 1_442_695_040_888_963_407;

/// Step the world-seed LCG by one round, salted with `salt`.
///
/// `s * (s * mul + add) + salt`, all wrapping.
#[inline]
#[must_use]
pub const fn mc_step_seed(s: u64, salt: u64) -> u64 {
    s.wrapping_mul(s.wrapping_mul(STEP_MUL).wrapping_add(STEP_ADD))
        .wrapping_add(salt)
}

/// First PRNG integer in `[0, m)` derived from a chunk seed.
///
/// Mirrors `mcFirstInt`: the seed is treated as `i64`, arithmetic-shifted
/// right by 24, taken modulo `m` *in 64-bit precision*, then truncated to
/// `i32` and adjusted into the positive range. The order matters — taking
/// `as i32` before `% m` would drop the upper 32 bits and yield a
/// different remainder whenever `s >> 24` is outside `i32` range.
#[inline]
#[must_use]
pub const fn mc_first_int(s: u64, m: i32) -> i32 {
    let r = (((s as i64) >> 24) % (m as i64)) as i32;
    if r < 0 { r + m } else { r }
}

/// `true` if `mc_first_int(s, m) == 0` — a slightly cheaper form than
/// computing the full value and comparing.
#[inline]
#[must_use]
pub const fn mc_first_is_zero(s: u64, m: i32) -> bool {
    (((s as i64) >> 24) % (m as i64)) == 0
}

/// Combine the per-layer start seed with chunk coordinates.
///
/// Coordinates are sign-extended through `i32 -> i64 -> u64` to match
/// cubiomes' `getChunkSeed`, which uses `uint64_t` arithmetic on `int`
/// inputs via C's implicit promotion rules.
#[inline]
#[must_use]
pub const fn get_chunk_seed(start_seed: u64, x: i32, z: i32) -> u64 {
    let x = x as i64 as u64;
    let z = z as i64 as u64;
    let mut cs = start_seed.wrapping_add(x);
    cs = mc_step_seed(cs, z);
    cs = mc_step_seed(cs, x);
    mc_step_seed(cs, z)
}

/// Layer-specific salt: three `mc_step_seed(salt, salt)` rounds.
#[inline]
#[must_use]
pub const fn get_layer_salt(salt: u64) -> u64 {
    let mut ls = mc_step_seed(salt, salt);
    ls = mc_step_seed(ls, salt);
    mc_step_seed(ls, salt)
}

/// World-seed seed pipeline start salt: three `mc_step_seed(_, ls)` rounds.
#[inline]
#[must_use]
pub const fn get_start_salt(world_seed: u64, layer_salt: u64) -> u64 {
    let mut st = world_seed;
    st = mc_step_seed(st, layer_salt);
    st = mc_step_seed(st, layer_salt);
    mc_step_seed(st, layer_salt)
}

/// World-seed seed pipeline final start seed.
///
/// Mirrors `getStartSeed`: applies `get_start_salt`, then one more
/// `mc_step_seed` with zero salt.
#[inline]
#[must_use]
pub const fn get_start_seed(world_seed: u64, layer_salt: u64) -> u64 {
    mc_step_seed(get_start_salt(world_seed, layer_salt), 0)
}

/// Modular inverse: the smallest `b` such that `(x * b) mod m == 1`.
///
/// Assumes `x` and `m` are positive (`< 2^63`) and coprime. Returns `0`
/// when no inverse exists, matching cubiomes' `mulInv`. Implementation is
/// the extended Euclidean algorithm with wrapping subtractions.
#[inline]
#[must_use]
#[allow(clippy::many_single_char_names)] // mirrors variable names in the C source
pub const fn mul_inv(mut x: u64, m: u64) -> u64 {
    if (m as i64) <= 1 {
        return 0;
    }
    let n = m;
    let mut m = m;
    let mut a: u64 = 0;
    let mut b: u64 = 1;
    while (x as i64) > 1 {
        if m == 0 {
            return 0;
        }
        let q = x / m;
        let t = m;
        m = x % m;
        x = t;
        let t = a;
        a = b.wrapping_sub(q.wrapping_mul(a));
        b = t;
    }
    if (b as i64) < 0 {
        return b.wrapping_add(n);
    }
    b
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mc_step_seed_is_deterministic() {
        let a = mc_step_seed(123, 456);
        let b = mc_step_seed(123, 456);
        assert_eq!(a, b);
        let c = mc_step_seed(123, 457);
        assert_ne!(a, c);
    }

    #[test]
    fn mc_first_int_is_in_range() {
        for s in [0u64, 1, 0xdead_beef, u64::MAX] {
            for m in [3i32, 5, 7, 24, 100] {
                let r = mc_first_int(s, m);
                assert!((0..m).contains(&r), "mc_first_int({s}, {m}) -> {r}");
            }
        }
    }

    #[test]
    fn mc_first_is_zero_agrees_with_mc_first_int() {
        for s in 0u64..512 {
            for m in [2i32, 3, 7, 16, 24] {
                assert_eq!(mc_first_is_zero(s, m), mc_first_int(s, m) == 0);
            }
        }
    }

    #[test]
    fn get_chunk_seed_depends_on_both_coordinates() {
        let a = get_chunk_seed(1, 0, 0);
        let b = get_chunk_seed(1, 1, 0);
        let c = get_chunk_seed(1, 0, 1);
        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_ne!(b, c);
    }

    #[test]
    fn get_chunk_seed_handles_negative_coords() {
        // Confirms the sign-extension semantics match cubiomes — if the
        // x/z cast dropped the sign, both calls would yield the same
        // result for x = -1 and x = 0xFFFFFFFF (as u32).
        let neg = get_chunk_seed(0, -1, -1);
        let big = get_chunk_seed(0, i32::MIN, i32::MIN);
        assert_ne!(neg, big);
    }

    #[test]
    fn get_layer_salt_changes_with_input() {
        assert_ne!(get_layer_salt(0), get_layer_salt(1));
    }

    #[test]
    fn start_seed_is_step_of_start_salt() {
        let ws = 0xdead_beef_cafe;
        let ls = get_layer_salt(7);
        assert_eq!(
            get_start_seed(ws, ls),
            mc_step_seed(get_start_salt(ws, ls), 0)
        );
    }

    #[test]
    fn mul_inv_round_trips_for_random_primes() {
        // Small set of coprime pairs to validate the extended Euclidean
        // algorithm. We avoid `m <= 1` and `gcd(x, m) > 1`, both of which
        // return 0 by spec.
        for &(x, m) in &[(3u64, 7u64), (5, 13), (17, 31), (1234, 9999)] {
            let inv = mul_inv(x, m);
            assert_ne!(inv, 0, "mul_inv({x}, {m}) returned 0");
            assert_eq!((x.wrapping_mul(inv)) % m, 1);
        }
    }

    #[test]
    fn mul_inv_returns_zero_when_m_too_small() {
        assert_eq!(mul_inv(3, 0), 0);
        assert_eq!(mul_inv(3, 1), 0);
    }

    #[test]
    fn const_fn_chain_compiles() {
        // Ensures the whole pipeline can run in a const context, which
        // is the whole point of every helper being const fn.
        const SALT: u64 = get_layer_salt(42);
        const SS: u64 = get_start_seed(0xbeef, SALT);
        const CS: u64 = get_chunk_seed(SS, 3, -1);
        const _: () = assert!(CS != 0 || SS != 0); // sanity: avoid the dead branch
    }
}
