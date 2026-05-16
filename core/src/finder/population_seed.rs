//! `getPopulationSeed` — chunk-local decorator-feature seed.
//!
//! Bit-exact port of cubiomes' `getPopulationSeed` from `finders.c`.
//! Used by every chunk-local feature (End Islands, Desert Wells,
//! Geodes, End Gateways, …) and by Bastion's 1.18+ chunk-generation
//! RNG. The version dispatch is:
//!
//! - MC ≥ 1.18: Xoroshiro128++ via `xSetSeed` / `xNextLongJ` twice.
//! - MC < 1.18: Java RNG via `setSeed` / `nextLong` twice.
//! - MC ≥ 1.13: both halves are OR'd with 1 (force odd).
//! - MC < 1.13: both halves go through `(int64_t) / 2 * 2 + 1`,
//!   which produces an odd value with floor-toward-zero semantics
//!   for negatives (slightly different from `| 1` when the low bit
//!   of a negative even value is 0).
//!
//! The final mix `(x * a + z * b) ^ ws` uses C's signed-to-unsigned
//! conversion: `int * uint64_t` sign-extends `x`/`z` through `i64`
//! before the multiply, so we mirror that with `as i64 as u64`.

use crate::mc_version::MCVersion;
use crate::rng::{JavaRng, Xoroshiro};

/// `getPopulationSeed(mc, ws, x, z)` — return the chunk-local
/// decorator RNG seed for the chunk whose block-origin is `(x, z)`.
#[must_use]
pub fn get_population_seed(mc: MCVersion, ws: u64, x: i32, z: i32) -> u64 {
    let (mut a, mut b) = if mc.is_at_least(MCVersion::V1_18) {
        let mut xr = Xoroshiro::new(ws);
        let a = xr.next_long_j();
        let b = xr.next_long_j();
        (a, b)
    } else {
        let mut s = JavaRng::new(ws);
        let a = s.next_long();
        let b = s.next_long();
        (a, b)
    };
    if mc.is_at_least(MCVersion::V1_13) {
        a |= 1;
        b |= 1;
    } else {
        // (int64_t)a / 2 * 2 + 1: truncates toward zero, then forces
        // the low bit to 1. For negatives this differs from `| 1`
        // only when the value is even (e.g. -4 → -3 here, but
        // -4 | 1 = -3 too — same on this leg; the differences show
        // up for odd negatives like -3 → -1 vs -3 | 1 = -3).
        a = ((a as i64) / 2 * 2) as u64;
        a = a.wrapping_add(1);
        b = ((b as i64) / 2 * 2) as u64;
        b = b.wrapping_add(1);
    }
    let xa = (x as i64 as u64).wrapping_mul(a);
    let zb = (z as i64 as u64).wrapping_mul(b);
    xa.wrapping_add(zb) ^ ws
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_for_same_inputs() {
        let a = get_population_seed(MCVersion::V1_18, 0xdead_beef, 12, 34);
        let b = get_population_seed(MCVersion::V1_18, 0xdead_beef, 12, 34);
        assert_eq!(a, b);
    }

    #[test]
    fn version_dispatch_differs() {
        // For non-zero (x, z) the chosen RNG path leaks through `a`/`b`.
        // At (0, 0) the mix degenerates to `ws`, so use 12, 34 instead.
        let modern = get_population_seed(MCVersion::V1_18, 1, 12, 34);
        let pre18 = get_population_seed(MCVersion::V1_17, 1, 12, 34);
        assert_ne!(modern, pre18);
    }
}
