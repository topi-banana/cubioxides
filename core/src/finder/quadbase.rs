//! Quad-structure base checks (witch huts, pyramids, …).
//!
//! Ports cubiomes' `isQuadBaseFeature24Classic` /
//! `isQuadBaseFeature24` / `getQuadHutCst` from `quadbase.{c,h}`.
//! These are the fast-path filters used by the multi-threaded
//! `searchAll48` to harvest quad-witch-hut candidates from 48-bit
//! seed space.

use super::StructureConfig;

/// Lower-20-bit "ideal" quad-structure constellations (cubiomes'
/// `low20QuadIdeal`).
pub const LOW20_QUAD_IDEAL: &[u64] = &[0x43f18, 0xc751a, 0xf520a];

/// Lower-20-bit "classic" quad-structure constellations (cubiomes'
/// `low20QuadClassic`).
pub const LOW20_QUAD_CLASSIC: &[u64] = &[0x43f18, 0x79a0a, 0xc751a, 0xf520a];

/// Lower-20-bit constellations for normal quad-witch-hut farms
/// (cubiomes' `low20QuadHutNormal`).
pub const LOW20_QUAD_HUT_NORMAL: &[u64] = &[
    0x43f18, 0x65118, 0x75618, 0x79a0a, 0x89718, 0x9371a, 0xa5a08, 0xb5e18, 0xc751a, 0xf520a,
];

/// Lower-20-bit constellations for "barely" quad-witch-hut farms.
pub const LOW20_QUAD_HUT_BARELY: &[u64] = &[
    0x1272d, 0x17908, 0x367b9, 0x43f18, 0x487c9, 0x487ce, 0x50aa7, 0x647b5, 0x65118, 0x75618,
    0x79a0a, 0x89718, 0x9371a, 0x967ec, 0xa3d0a, 0xa5918, 0xa591d, 0xa5a08, 0xb5e18, 0xc6749,
    0xc6d9a, 0xc751a, 0xd7108, 0xd717a, 0xe2739, 0xe9918, 0xee1c4, 0xf520a,
];

/// Classification returned by [`get_quad_hut_cst`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
#[allow(missing_docs)]
pub enum QuadHutCst {
    None = 0,
    Ideal = 1,
    Classic = 2,
    Normal = 3,
    Barely = 4,
}

/// Cubiomes' `getQuadHutCst(low20)` — classify the lower 20 bits of
/// a witch-hut quad-base seed against the canonical constellation
/// tables.
#[must_use]
pub fn get_quad_hut_cst(low20: u64) -> QuadHutCst {
    if LOW20_QUAD_IDEAL.contains(&low20) {
        QuadHutCst::Ideal
    } else if LOW20_QUAD_CLASSIC.contains(&low20) {
        QuadHutCst::Classic
    } else if LOW20_QUAD_HUT_NORMAL.contains(&low20) {
        QuadHutCst::Normal
    } else if LOW20_QUAD_HUT_BARELY.contains(&low20) {
        QuadHutCst::Barely
    } else {
        QuadHutCst::None
    }
}

// Cubiomes' `JAVA_NEXT_INT24` macro inlined.
#[inline]
fn java_next_int_24(s: &mut u64) -> i32 {
    let mut a: u64 = (1u64 << 48) - 1;
    let mut c: u64 = 0x0005_deec_e66d_u64.wrapping_mul(*s);
    c = c.wrapping_add(11);
    a &= c;
    *s = a;
    a = (a as i64 >> 17) as u64;
    c = 0xaaaa_aaab_u64.wrapping_mul(a);
    c = (c as i64 >> 36) as u64;
    (a as i32) - ((c << 3) as i32) * 3
}

const K: u64 = 0x0005_deec_e66d;

/// `isQuadBaseFeature24Classic(sconf, seed)` — return `true` iff
/// the lower 48 bits of `seed` produce one of cubiomes' "classic"
/// quad-witch-hut constellations. (Cubiomes returns `1.0f` here;
/// Rust returns a bool since the radius value isn't computed.)
#[must_use]
pub fn is_quad_base_feature_24_classic(sconf: StructureConfig, seed: u64) -> bool {
    let seed = seed.wrapping_add(sconf.salt as i64 as u64);
    let mut s00 = seed;
    let mut s11 = 341_873_128_712_u64
        .wrapping_add(132_897_987_541)
        .wrapping_add(seed);

    s00 ^= K;
    if java_next_int_24(&mut s00) < 22 {
        return false;
    }
    if java_next_int_24(&mut s00) < 22 {
        return false;
    }

    s11 ^= K;
    if java_next_int_24(&mut s11) > 1 {
        return false;
    }
    if java_next_int_24(&mut s11) > 1 {
        return false;
    }

    let mut s01 = 341_873_128_712_u64.wrapping_add(seed);
    let mut s10 = 132_897_987_541_u64.wrapping_add(seed);

    s01 ^= K;
    if java_next_int_24(&mut s01) > 1 {
        return false;
    }
    if java_next_int_24(&mut s01) < 22 {
        return false;
    }

    s10 ^= K;
    if java_next_int_24(&mut s10) < 22 {
        return false;
    }
    if java_next_int_24(&mut s10) > 1 {
        return false;
    }

    true
}

/// `isQuadBaseFeature24(sconf, seed, ax, ay, az)` — exact-radius
/// quad-structure filter for the `regionSize = 32, chunkRange = 24,
/// radius = 128` configuration. Returns the enclosing-sphere radius
/// (in blocks) on success, or `None` if `seed` doesn't qualify.
#[must_use]
pub fn is_quad_base_feature_24(
    sconf: StructureConfig,
    seed: u64,
    ax: i32,
    ay: i32,
    az: i32,
) -> Option<f32> {
    let seed = seed.wrapping_add(sconf.salt as i64 as u64);
    let mut s00 = seed;
    let mut s11 = 341_873_128_712_u64
        .wrapping_add(132_897_987_541)
        .wrapping_add(seed);

    s00 ^= K;
    let x0 = java_next_int_24(&mut s00);
    if x0 < 20 {
        return None;
    }
    let z0 = java_next_int_24(&mut s00);
    if z0 < 20 {
        return None;
    }

    s11 ^= K;
    let x1 = java_next_int_24(&mut s11);
    if x1 > x0 - 20 {
        return None;
    }
    let z1 = java_next_int_24(&mut s11);
    if z1 > z0 - 20 {
        return None;
    }

    let x = x1 + 32 - x0;
    let z = z1 + 32 - z0;
    if x * x + z * z > 255 {
        return None;
    }

    let mut s01 = 341_873_128_712_u64.wrapping_add(seed);
    let mut s10 = 132_897_987_541_u64.wrapping_add(seed);

    s01 ^= K;
    let x2 = java_next_int_24(&mut s01);
    if x2 >= 4 {
        return None;
    }
    let z2 = java_next_int_24(&mut s01);
    if z2 < 20 {
        return None;
    }

    s10 ^= K;
    let x3 = java_next_int_24(&mut s10);
    if x3 < 20 {
        return None;
    }
    let z3 = java_next_int_24(&mut s10);
    if z3 >= 4 {
        return None;
    }

    let x = x2 + 32 - x3;
    let z = z3 + 32 - z2;
    if x * x + z * z > 255 {
        return None;
    }

    let radius = get_enclosing_radius(x0, z0, x1, z1, x2, z2, x3, z3, ax, ay, az, 32, 128);
    if radius < 128.0 { Some(radius) } else { None }
}

/// Cubiomes' static `getEnclosingRadius` — brute-force the optimal
/// AFK position for the four corner structures and return the
/// enclosing-sphere radius in blocks.
#[allow(clippy::too_many_arguments)]
fn get_enclosing_radius(
    x0: i32,
    z0: i32,
    x1: i32,
    z1: i32,
    x2: i32,
    z2: i32,
    x3: i32,
    z3: i32,
    ax: i32,
    ay: i32,
    az: i32,
    reg: i32,
    gap: i32,
) -> f32 {
    // chunks → blocks
    let x0 = x0 << 4;
    let z0 = z0 << 4;
    let x1 = ((reg + x1) << 4) + ax;
    let z1 = ((reg + z1) << 4) + az;
    let x2 = ((reg + x2) << 4) + ax;
    let z2 = z2 << 4;
    let x3 = x3 << 4;
    let z3 = ((reg + z3) << 4) + az;

    let mut sqrad: i32 = 0x7fff_ffff;

    let cbx0 = x1.max(x2) - gap;
    let cbz0 = z1.max(z3) - gap;
    let cbx1 = x0.min(x3) + gap;
    let cbz1 = z0.min(z2) + gap;

    for z in cbz0..=cbz1 {
        for x in cbx0..=cbx1 {
            let mut sq = 0_i32;
            for (tx, tz) in [(x0, z0), (x1, z1), (x2, z2), (x3, z3)] {
                let s = (x - tx) * (x - tx) + (z - tz) * (z - tz);
                if s > sq {
                    sq = s;
                }
            }
            if sq < sqrad {
                sqrad = sq;
            }
        }
    }

    if sqrad < 0x7fff_ffff {
        ((sqrad as f32) + (ay * ay) as f32 / 4.0).sqrt()
    } else {
        0xffff as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::finder::{StructureType, get_structure_config};
    use crate::mc_version::MCVersion;

    #[test]
    fn get_quad_hut_cst_categorises() {
        assert_eq!(get_quad_hut_cst(0x43f18), QuadHutCst::Ideal);
        assert_eq!(get_quad_hut_cst(0x79a0a), QuadHutCst::Classic);
        assert_eq!(get_quad_hut_cst(0x65118), QuadHutCst::Normal);
        assert_eq!(get_quad_hut_cst(0x1272d), QuadHutCst::Barely);
        assert_eq!(get_quad_hut_cst(0xdead_beef), QuadHutCst::None);
    }

    #[test]
    fn random_seed_is_not_a_quad_base() {
        let sconf = get_structure_config(StructureType::SwampHut, MCVersion::V1_18).unwrap();
        // A random seed almost certainly fails the classic check.
        assert!(!is_quad_base_feature_24_classic(sconf, 0xdead_beef));
        assert!(is_quad_base_feature_24(sconf, 0xdead_beef, 0, 0, 0).is_none());
    }
}
