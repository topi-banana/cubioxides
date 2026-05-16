//! Quad-structure base checks (witch huts, pyramids, …).
//!
//! Ports cubiomes' `isQuadBaseFeature24Classic` /
//! `isQuadBaseFeature24` / `getQuadHutCst` from `quadbase.{c,h}`.
//! These are the fast-path filters used by the multi-threaded
//! `searchAll48` to harvest quad-witch-hut candidates from 48-bit
//! seed space.

use super::{Pos, StructureConfig};

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

/// Cubiomes' static `blocksInRange`: count player-foot block cells
/// at `(x..x+ax, z..z+az)` whose Euclidean distance² from any of
/// `p`'s `ax × az` footprints fits in `rsq`.
fn blocks_in_range(p: &[Pos], x: i64, z: i64, ax: i32, az: i32, rsq: f64) -> i32 {
    let mut cnt = 0_i32;
    for entry in p {
        let dx = entry.x as f64 - x as f64;
        let dz = entry.z as f64 - z as f64;
        for px in 0..ax {
            for pz in 0..az {
                let ddx = px as f64 + dx;
                let ddz = pz as f64 + dz;
                if ddx * ddx + ddz * ddz <= rsq {
                    cnt += 1;
                }
            }
        }
    }
    cnt
}

/// Mutable per-flood-fill state. Mirrors cubiomes' `afk_meta_t`.
struct AfkMeta<'a> {
    p: &'a [Pos],
    buf: Vec<i32>,
    x0: i64,
    z0: i64,
    w: i64,
    h: i64,
    ax: i32,
    az: i32,
    rsq: f64,
    best: i32,
    sumn: i32,
    sumx: i64,
    sumz: i64,
}

/// Cubiomes' static recursive `checkAfkDist`. 8-way flood fill over
/// the `(w, h)` grid that updates `best`/`sumn`/`sumx`/`sumz` for
/// every cell whose `blocks_in_range` count matches `best`.
fn check_afk_dist(d: &mut AfkMeta<'_>, x: i64, z: i64) {
    if x < 0 || z < 0 || x >= d.w || z >= d.h {
        return;
    }
    let idx = (z * d.w + x) as usize;
    if d.buf[idx] != 0 {
        return;
    }
    let q = blocks_in_range(d.p, x + d.x0, z + d.z0, d.ax, d.az, d.rsq);
    d.buf[idx] = q;
    if q >= d.best {
        if q > d.best {
            d.best = q;
            d.sumn = 1;
            d.sumx = d.x0 + x;
            d.sumz = d.z0 + z;
        } else {
            d.sumn += 1;
            d.sumx += d.x0 + x;
            d.sumz += d.z0 + z;
        }
        check_afk_dist(d, x, z - 1);
        check_afk_dist(d, x, z + 1);
        check_afk_dist(d, x - 1, z);
        check_afk_dist(d, x + 1, z);
        check_afk_dist(d, x - 1, z - 1);
        check_afk_dist(d, x - 1, z + 1);
        check_afk_dist(d, x + 1, z - 1);
        check_afk_dist(d, x + 1, z + 1);
    }
}

/// `getOptimalAfk(p, ax, ay, az, spcnt)` — find the AFK position
/// inside a quad-structure footprint that maximises the number of
/// blocks within the 128-block player-spawn sphere.
///
/// Bit-exact port of cubiomes' `getOptimalAfk`. Returns the optimal
/// `(x, z)` AFK block position; when `spcnt` is `Some`, writes the
/// achieved in-range block count into it.
#[must_use]
#[allow(clippy::too_many_lines, clippy::needless_range_loop)]
pub fn get_optimal_afk(p: &[Pos; 4], ax: i32, ay: i32, az: i32, spcnt: Option<&mut i32>) -> Pos {
    let mut min_x: i64 = i64::MAX;
    let mut min_z: i64 = i64::MAX;
    let mut max_x: i64 = i64::MIN;
    let mut max_z: i64 = i64::MIN;
    for entry in p {
        let x = entry.x as i64;
        let z = entry.z as i64;
        if x < min_x {
            min_x = x;
        }
        if z < min_z {
            min_z = z;
        }
        if x > max_x {
            max_x = x;
        }
        if z > max_z {
            max_z = z;
        }
    }
    min_x += i64::from(ax / 2);
    min_z += i64::from(az / 2);
    max_x += i64::from(ax / 2);
    max_z += i64::from(az / 2);

    let rsq = 128.0_f64 * 128.0 - (ay as f64) * (ay as f64) / 4.0;
    let w = max_x - min_x;
    let h = max_z - min_z;
    let mut afk = Pos {
        x: p[0].x + ax / 2,
        z: p[0].z + az / 2,
    };
    let mut cnt = ax * az;

    let mut d = AfkMeta {
        p,
        buf: vec![0_i32; (w * h) as usize],
        x0: min_x,
        z0: min_z,
        w,
        h,
        ax,
        az,
        rsq,
        best: 0,
        sumn: 0,
        sumx: 0,
        sumz: 0,
    };

    // 6 starting midpoints (the 4 quad-pair midpoints plus the two
    // diagonal midpoints).
    let dsp: [Pos; 6] = [
        Pos {
            x: (p[0].x + p[2].x) / 2,
            z: (p[0].z + p[2].z) / 2,
        },
        Pos {
            x: (p[1].x + p[3].x) / 2,
            z: (p[1].z + p[3].z) / 2,
        },
        Pos {
            x: (p[0].x + p[1].x) / 2,
            z: (p[0].z + p[1].z) / 2,
        },
        Pos {
            x: (p[2].x + p[3].x) / 2,
            z: (p[2].z + p[3].z) / 2,
        },
        Pos {
            x: (p[0].x + p[3].x) / 2,
            z: (p[0].z + p[3].z) / 2,
        },
        Pos {
            x: (p[1].x + p[2].x) / 2,
            z: (p[1].z + p[2].z) / 2,
        },
    ];
    let mut v = [0_i32; 6];
    for (i, midp) in dsp.iter().enumerate() {
        v[i] = blocks_in_range(p, midp.x as i64, midp.z as i64, ax, az, rsq);
    }

    for _ in 0..6 {
        let mut jmax = 0_usize;
        let mut vmax = 0_i32;
        for j in 0..6 {
            if v[j] > vmax {
                jmax = j;
                vmax = v[j];
            }
        }
        if vmax <= ax * az {
            break;
        }
        d.best = vmax;
        d.sumn = 0;
        d.sumx = 0;
        d.sumz = 0;
        let start_x = dsp[jmax].x as i64 - d.x0;
        let start_z = dsp[jmax].z as i64 - d.z0;
        check_afk_dist(&mut d, start_x, start_z);
        if d.best > cnt {
            cnt = d.best;
            if d.sumn == 0 {
                // cubiomes hits `(int) round(0.0 / 0.0)` here when
                // the starting midpoint is OOB of the `+ax/2`-shifted
                // bounding box (common: it happens whenever two
                // anchor points share an x or z coordinate that
                // equals min/max). x86 `cvttsd2si NaN` returns
                // `INT_MIN`; mirror that bit-pattern so our output
                // matches cubiomes on x86.
                afk.x = i32::MIN;
                afk.z = i32::MIN;
            } else {
                afk.x = (d.sumx as f64 / d.sumn as f64).round() as i32;
                afk.z = (d.sumz as f64 / d.sumn as f64).round() as i32;
            }
            if cnt >= 3 * ax * az {
                break;
            }
        }
        v[jmax] = 0;
    }

    if let Some(out) = spcnt {
        *out = cnt;
    }
    afk
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
