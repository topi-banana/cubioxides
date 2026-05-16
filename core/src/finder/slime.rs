//! `isSlimeChunk` — the per-chunk slime-spawn predicate.
//!
//! Bit-exact port of the inline `isSlimeChunk` in
//! `cubiomes/finders.h`. The function mixes the world seed with a
//! polynomial of the chunk coordinates, XORs in a constant, runs
//! a single `nextInt(10)` through a Java RNG, and returns true if
//! the result is zero.

use crate::rng::JavaRng;

/// Returns `true` iff the chunk at `(chunk_x, chunk_z)` is a slime
/// chunk for the given world `seed`.
///
/// # Example
///
/// ```
/// use cubioxides::finder::is_slime_chunk;
///
/// // Cubiomes' deterministic slime-chunk classifier — same answer
/// // for the same (seed, chunk_x, chunk_z) on every platform.
/// // About 10 % of chunks at any given seed are slime chunks.
/// let _is_slime = is_slime_chunk(0xdead_beef, 0, 0);
/// ```
#[must_use]
pub fn is_slime_chunk(seed: u64, chunk_x: i32, chunk_z: i32) -> bool {
    // cubiomes' arithmetic mixes 32-bit and 64-bit signed operands;
    // mirror it exactly to preserve sign-extension behaviour.
    let mut rnd = seed;
    rnd = rnd.wrapping_add(chunk_x.wrapping_mul(0x005a_c0db) as i64 as u64);
    rnd = rnd.wrapping_add(chunk_x.wrapping_mul(chunk_x).wrapping_mul(0x004c_1906) as i64 as u64);
    rnd = rnd.wrapping_add(chunk_z.wrapping_mul(0x0005_f24f) as i64 as u64);
    // NOTE: cubiomes' literal `0x4307a7ULL` forces the multiply into
    // 64-bit arithmetic — the (chunkZ * chunkZ) factor wraps in 32
    // bits, then sign-extends to int64 before the multiply.
    let zsq = chunk_z.wrapping_mul(chunk_z);
    rnd = rnd.wrapping_add((zsq as i64).wrapping_mul(0x0043_07a7) as u64);
    rnd ^= 0x3ad8_025f;
    let mut rng = JavaRng::new(rnd);
    rng.next_int(10) == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        let a = is_slime_chunk(0xdead_beef, 3, -7);
        let b = is_slime_chunk(0xdead_beef, 3, -7);
        assert_eq!(a, b);
    }

    #[test]
    fn distinct_seeds_disagree_somewhere() {
        // Sample 10 chunks; at least one should differ between two
        // unrelated seeds.
        let mut delta = 0;
        for x in 0..5 {
            for z in 0..5 {
                if is_slime_chunk(1, x, z) != is_slime_chunk(2, x, z) {
                    delta += 1;
                }
            }
        }
        assert!(delta > 0, "no chunk differed between seed 1 and seed 2");
    }
}
