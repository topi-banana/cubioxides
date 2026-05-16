//! Mineshaft locator — `getMineshafts(mc, seed, cx0, cz0, cx1, cz1)`.
//!
//! Bit-exact port of cubiomes' loop over a rectangular chunk
//! region, testing each chunk's 1/250 mineshaft probability. The
//! 1.13+ check is a simple `nextDouble < 0.004` against a
//! chunk-coordinate-mixed seed; pre-1.13 also gates on a
//! distance-from-origin probability.

#![allow(clippy::many_single_char_names)]

use crate::finder::Pos;
use crate::mc_version::MCVersion;
use crate::rng::JavaRng;

/// Cubiomes' `getMineshafts(mc, seed, cx0, cz0, cx1, cz1, out, nout)`.
/// Returns up to `n_max` mineshaft chunk positions (in block
/// coordinates) within the inclusive chunk-coordinate rectangle
/// `[(cx0, cz0), (cx1, cz1)]`.
///
/// # Example
///
/// ```
/// use cubioxides::MCVersion;
/// use cubioxides::finder::get_mineshafts;
///
/// // Scan an 80×80 chunk window for mineshafts on a 1.18 seed.
/// // Each chunk has a ~0.4% spawn rate so the expected count is
/// // about 25 across the window. Results are bit-identical to
/// // cubiomes for the same (mc, seed, region).
/// let positions = get_mineshafts(MCVersion::V1_18, 0xdead_beef, -40, -40, 40, 40, 256);
/// // Positions are block-coordinates (chunk_x * 16, chunk_z * 16).
/// for p in &positions {
///     assert_eq!(p.x.rem_euclid(16), 0);
///     assert_eq!(p.z.rem_euclid(16), 0);
/// }
/// ```
#[must_use]
pub fn get_mineshafts(
    mc: MCVersion,
    seed: u64,
    cx0: i32,
    cz0: i32,
    cx1: i32,
    cz1: i32,
    n_max: usize,
) -> Vec<Pos> {
    let mut rng = JavaRng::new(seed);
    let a = rng.next_long();
    let b = rng.next_long();
    let mut out = Vec::new();

    for i in cx0..=cx1 {
        // cubiomes: `aix = i * a ^ seed` — `i * a` wraps in 64 bits
        // because `a` is uint64_t; the `i` operand promotes to
        // int64 (sign-extended) then to uint64.
        let i64u = i as i64 as u64;
        let aix = i64u.wrapping_mul(a) ^ seed;

        for j in cz0..=cz1 {
            let j64u = j as i64 as u64;
            let mixed = aix ^ j64u.wrapping_mul(b);
            let mut s = JavaRng::new(mixed);

            if mc.is_at_least(MCVersion::V1_13) {
                if s.next_double() < 0.004 {
                    if out.len() < n_max {
                        out.push(Pos {
                            x: i * 16,
                            z: j * 16,
                        });
                    } else {
                        // Cubiomes stops appending but keeps
                        // counting. Our Vec sticks to the cap.
                    }
                }
            } else {
                // skipNextN(s, 1) advances one step.
                s.next(32);
                if s.next_double() < 0.004 {
                    // Pre-1.13 also requires a distance check.
                    let d = i.abs().max(j.abs());
                    let accept = if d >= 80 { true } else { s.next_int(80) < d };
                    if accept && out.len() < n_max {
                        out.push(Pos {
                            x: i * 16,
                            z: j * 16,
                        });
                    }
                }
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        let a = get_mineshafts(MCVersion::V1_18, 12345, -10, -10, 10, 10, 200);
        let b = get_mineshafts(MCVersion::V1_18, 12345, -10, -10, 10, 10, 200);
        assert_eq!(a, b);
    }

    #[test]
    fn density_is_below_one_percent() {
        // ~0.4% expected: in a 100x100 chunk window we should see
        // 30-50 mineshafts on a typical seed.
        let v = get_mineshafts(MCVersion::V1_18, 0xdead_beef, 0, 0, 99, 99, 10_000);
        let area = 100 * 100;
        assert!(
            v.len() < area / 50,
            "unexpectedly many mineshafts: {}",
            v.len()
        );
    }
}
