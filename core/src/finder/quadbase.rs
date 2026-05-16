//! Quad-structure base checks (witch huts, pyramids, …).
//!
//! Ports cubiomes' `isQuadBaseFeature24Classic` /
//! `isQuadBaseFeature24` / `getQuadHutCst` from `quadbase.{c,h}`.
//! These are the fast-path filters used by the multi-threaded
//! `searchAll48` to harvest quad-witch-hut candidates from 48-bit
//! seed space.

#![allow(clippy::doc_markdown, clippy::items_after_statements)]

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
///
/// # Example
///
/// ```
/// use cubioxides::MCVersion;
/// use cubioxides::finder::{StructureType, get_structure_config, is_quad_base_feature_24};
///
/// // Lower-level swamp-hut quad-base check used by `is_quad_base`
/// // when `radius == 128`. The `(ax, ay, az)` bbox is the spawn box
/// // — `(8, 8, 10)` matches cubiomes' SwampHut footprint with the
/// // +1 inclusive margin already applied.
/// let sconf = get_structure_config(StructureType::SwampHut, MCVersion::V1_18)
///     .expect("SwampHut config exists for 1.18");
/// let _maybe = is_quad_base_feature_24(sconf, 0xdead_beef, 8, 8, 10);
/// ```
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

/// `isQuadBaseFeature(sconf, seed, ax, ay, az, radius)` — generic
/// radius-parameterised quad-structure filter. Used for non-128
/// radii or non-(R=32, C=24) configs (e.g., Outpost with
/// `R=32, C=24`, Ocean_Ruin / Shipwreck at non-standard radii).
///
/// Returns `Some(sqrad)` on success, `None` otherwise.
#[must_use]
#[allow(clippy::many_single_char_names)]
pub fn is_quad_base_feature(
    sconf: StructureConfig,
    seed: u64,
    ax: i32,
    ay: i32,
    az: i32,
    radius: i32,
) -> Option<f32> {
    let seed = seed.wrapping_add(sconf.salt as i64 as u64);
    let s00 = seed;
    let s11 = 341_873_128_712_u64
        .wrapping_add(132_897_987_541)
        .wrapping_add(seed);
    const M: u64 = (1u64 << 48) - 1;
    const B: u64 = 0xb;

    let r = i32::from(sconf.region_size);
    let c = i32::from(sconf.chunk_range);
    let cd = radius / 8;
    let rm = r - ((cd * cd - (r - c + 1) * (r - c + 1)) as f32).sqrt() as i32;

    // Helper: step seed by one LCG iter, return next_int(C).
    let step = |s: &mut u64| -> i32 {
        *s = s.wrapping_mul(K).wrapping_add(B) & M;
        ((*s >> 17) as i32) % c
    };

    let mut s = s00 ^ K;
    let x0 = step(&mut s);
    if x0 <= rm {
        return None;
    }
    let z0 = step(&mut s);
    if z0 <= rm {
        return None;
    }

    let mut s = s11 ^ K;
    let x1 = step(&mut s);
    if x1 >= x0 - rm {
        return None;
    }
    let z1 = step(&mut s);
    if z1 >= z0 - rm {
        return None;
    }

    let x = x1 + r - x0;
    let z = z1 + r - z0;
    if x * x + z * z > cd * cd {
        return None;
    }

    let s01 = 341_873_128_712_u64.wrapping_add(seed);
    let s10 = 132_897_987_541_u64.wrapping_add(seed);

    let mut s = s01 ^ K;
    let x2 = step(&mut s);
    if x2 >= c - rm {
        return None;
    }
    let z2 = step(&mut s);
    if z2 <= rm {
        return None;
    }

    let mut s = s10 ^ K;
    let x3 = step(&mut s);
    if x3 <= rm {
        return None;
    }
    let z3 = step(&mut s);
    if z3 >= c - rm {
        return None;
    }

    let x = x2 + r - x3;
    let z = z3 + r - z2;
    if x * x + z * z > cd * cd {
        return None;
    }

    let sqrad = get_enclosing_radius(x0, z0, x1, z1, x2, z2, x3, z3, ax, ay, az, r, radius);
    if sqrad < radius as f32 {
        Some(sqrad)
    } else {
        None
    }
}

/// `isQuadBaseLarge(sconf, seed, ax, ay, az, radius)` — quad-base
/// filter for large structures (Monument). Each region's chunk
/// offset is the average of two `nextInt(C)` rolls (matching
/// `getLargeStructureChunkInRegion`), so each quadrant consumes 4
/// `nextInt` calls instead of 2.
///
/// Bit-exact port of cubiomes' static inline.
#[must_use]
#[allow(clippy::many_single_char_names)]
pub fn is_quad_base_large(
    sconf: StructureConfig,
    seed: u64,
    ax: i32,
    ay: i32,
    az: i32,
    radius: i32,
) -> Option<f32> {
    const M: u64 = (1u64 << 48) - 1;
    const B: u64 = 0xb;

    let seed = seed.wrapping_add(sconf.salt as i64 as u64);
    let s00 = seed;
    let s01 = 341_873_128_712_u64.wrapping_add(seed);
    let s10 = 132_897_987_541_u64.wrapping_add(seed);
    let s11 = 341_873_128_712_u64
        .wrapping_add(132_897_987_541)
        .wrapping_add(seed);

    let r = i32::from(sconf.region_size);
    let c = i32::from(sconf.chunk_range);
    // Cubiomes: `rm = 2*R + (min(ax, az) - 2*radius + 7) / 8`.
    let rm = 2 * r + (ax.min(az) - 2 * radius + 7) / 8;

    // Each quadrant draws 2 nextInt(C) values and sums them.
    let pair = |s: &mut u64| -> i32 {
        *s = s.wrapping_mul(K).wrapping_add(B) & M;
        let p1 = ((*s >> 17) as i32) % c;
        *s = s.wrapping_mul(K).wrapping_add(B) & M;
        let p2 = ((*s >> 17) as i32) % c;
        p1 + p2
    };

    let mut s = s00 ^ K;
    let x0 = pair(&mut s);
    if x0 <= rm {
        return None;
    }
    let z0 = pair(&mut s);
    if z0 <= rm {
        return None;
    }

    let mut s = s11 ^ K;
    let x1 = pair(&mut s);
    if x1 > x0 - rm {
        return None;
    }
    let z1 = pair(&mut s);
    if z1 > z0 - rm {
        return None;
    }

    // Pre-check: half-differences² ≤ 4 * radius².
    let dx = (x1 - x0) >> 1;
    let dz = (z1 - z0) >> 1;
    let dist_sq = (dx as i64) * (dx as i64) + (dz as i64) * (dz as i64);
    if dist_sq > (4 * radius * radius) as i64 {
        return None;
    }

    let mut s = s01 ^ K;
    let x2 = pair(&mut s);
    if x2 > x0 - rm {
        return None;
    }
    let z2 = pair(&mut s);
    if z2 <= rm {
        return None;
    }

    let mut s = s10 ^ K;
    let x3 = pair(&mut s);
    if x3 <= rm {
        return None;
    }
    let z3 = pair(&mut s);
    if z3 > z0 - rm {
        return None;
    }

    let sqrad = get_enclosing_radius(
        x0 >> 1,
        z0 >> 1,
        x1 >> 1,
        z1 >> 1,
        x2 >> 1,
        z2 >> 1,
        x3 >> 1,
        z3 >> 1,
        ax,
        ay,
        az,
        r,
        radius,
    );
    if sqrad < radius as f32 {
        Some(sqrad)
    } else {
        None
    }
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

/// `isQuadBase(sconf, seed, radius)` — type-aware dispatcher for
/// quad-base detection. Returns `Some(sqrad)` on a quad-base hit
/// (the squared bounding-sphere radius in blocks), `None`
/// otherwise.
///
/// Covers every structure type cubiomes' `isQuadBase` does:
/// `SwampHut` (uses `isQuadBaseFeature24` when `radius == 128`,
/// `isQuadBaseFeature` otherwise), `DesertPyramid` / `JungleTemple`
/// / `Igloo` / `Village` (same predicate, `(0, 0, 0)` bbox),
/// `Outpost` (`(72, 54, 72)` bbox), `OceanRuin` / `Shipwreck` /
/// `RuinedPortal` (`(0, 0, 0)` bbox via `isQuadBaseFeature`), and
/// `Monument` (uses `isQuadBaseLarge` with `(58, 23, 58)` bbox).
/// Panics for any other structure type — cubiomes prints an error
/// and `exit()`s.
///
/// # Example
///
/// ```
/// use cubioxides::MCVersion;
/// use cubioxides::finder::{StructureType, get_structure_config, is_quad_base};
///
/// // Probe an arbitrary world seed for a swamp-hut quad-base
/// // candidate. A `Some(sqrad)` return means the four region-seeds
/// // align tightly enough that all four huts can co-spawn within
/// // `sqrad` blocks² of a common AFK point.
/// let sconf = get_structure_config(StructureType::SwampHut, MCVersion::V1_18)
///     .expect("SwampHut config exists for 1.18");
/// let _maybe_quad = is_quad_base(sconf, 0xdead_beef, 128);
/// ```
#[must_use]
pub fn is_quad_base(sconf: StructureConfig, seed: u64, radius: i32) -> Option<f32> {
    use crate::finder::StructureType;
    let Some(ty) = StructureType::from_ord(sconf.struct_type as i32) else {
        panic!("is_quad_base: unknown struct_type {}", sconf.struct_type);
    };
    match ty {
        StructureType::SwampHut => {
            // Cubiomes' isQuadBase passes (7+1, 7+1, 9+1).
            if radius == 128 {
                is_quad_base_feature_24(sconf, seed, 8, 8, 10)
            } else {
                is_quad_base_feature(sconf, seed, 8, 8, 10, radius)
            }
        }
        StructureType::DesertPyramid
        | StructureType::JungleTemple
        | StructureType::Igloo
        | StructureType::Village => {
            // Cubiomes' note: "nothing special spawns here, why
            // would you want these?" — kept for completeness.
            if radius == 128 {
                is_quad_base_feature_24(sconf, seed, 0, 0, 0)
            } else {
                is_quad_base_feature(sconf, seed, 0, 0, 0, radius)
            }
        }
        StructureType::Outpost => {
            // Outposts spawn 8 chunks apart so perfect quad-outposts
            // don't exist; cubiomes still exposes the check.
            is_quad_base_feature(sconf, seed, 72, 54, 72, radius)
        }
        StructureType::OceanRuin | StructureType::Shipwreck | StructureType::RuinedPortal => {
            is_quad_base_feature(sconf, seed, 0, 0, 0, radius)
        }
        StructureType::Monument => {
            // Cubiomes' isQuadBaseLarge with (58, 23, 58) Monument bbox.
            is_quad_base_large(sconf, seed, 58, 23, 58, radius)
        }
        _ => {
            panic!(
                "is_quad_base: not implemented for structure type {ty:?} (cubiomes' isQuadBase prints an error and exits for these)"
            )
        }
    }
}

/// `scanForQuadBits` — sweep a `(w, h)` chunk-coordinate window for
/// quad-base candidates whose lower `lbitn` bits match `lbit`. Each
/// hit is appended to `qplist` as a chunk-coordinate `Pos`.
///
/// Bit-exact port of cubiomes' `scanForQuadBits`. The `inv_b` arg
/// is the modular inverse of `132897987541 mod 2^lbitn`.
///
/// Returns the number of hits found.
#[allow(clippy::too_many_arguments)]
pub fn scan_for_quad_bits(
    sconf: StructureConfig,
    radius: i32,
    s48: u64,
    lbit: u64,
    lbitn: u32,
    inv_b: u64,
    x: i64,
    z: i64,
    w: i64,
    h: i64,
    qplist: &mut Vec<Pos>,
    n: usize,
) -> usize {
    use crate::finder::move_structure;
    let m: u64 = 1u64 << lbitn;
    let a: u64 = 341_873_128_712;
    if n < 1 {
        return 0;
    }
    let lbit = lbit & (m - 1);

    let mut cnt = 0;
    for i in x..=(x + w) {
        let sx = s48.wrapping_add(a.wrapping_mul(i as u64));
        let mut j: i64 = ((z as u64 & !(m - 1))
            | ((lbit.wrapping_sub(sx)).wrapping_mul(inv_b) & (m - 1)))
            as i64;
        if j < z {
            j = j.wrapping_add(m as i64);
        }
        while j <= z + h {
            let sp = move_structure(s48, -(i as i32), -(j as i32));
            if (sp & (m - 1)) == lbit && is_quad_base(sconf, sp, radius).is_some() {
                qplist.push(Pos {
                    x: i as i32,
                    z: j as i32,
                });
                cnt += 1;
                if cnt >= n {
                    return cnt;
                }
            }
            j = j.wrapping_add(m as i64);
        }
    }
    cnt
}

/// `scanForQuads` — outer driver that runs `scan_for_quad_bits`
/// for every entry in `low_bits` (typically `LOW20_QUAD_*`). Stops
/// when `qplist.len() >= n`. Each entry past zero in `low_bits` is
/// treated as a sentinel terminator (matches cubiomes' `for (i = 0;
/// lowBits[i]; i++)`).
#[allow(clippy::too_many_arguments)]
pub fn scan_for_quads(
    sconf: StructureConfig,
    radius: i32,
    s48: u64,
    low_bits: &[u64],
    lbitn: u32,
    salt: u64,
    x: i64,
    z: i64,
    w: i64,
    h: i64,
    qplist: &mut Vec<Pos>,
    n: usize,
) -> usize {
    use crate::rng::mc_seed::mul_inv;
    let inv_b: u64 = if lbitn == 20 {
        132_477
    } else if lbitn == 48 {
        211_541_297_333_629
    } else {
        mul_inv(132_897_987_541, 1u64 << lbitn)
    };
    let mut cnt = 0;
    for &lb in low_bits {
        if lb == 0 {
            break;
        }
        cnt += scan_for_quad_bits(
            sconf,
            radius,
            s48,
            lb.wrapping_sub(salt),
            lbitn,
            inv_b,
            x,
            z,
            w,
            h,
            qplist,
            n - cnt,
        );
        if cnt >= n {
            break;
        }
    }
    cnt
}

/// Parallel variant of [`search_all_48`]. Splits the seed range
/// evenly into `n_chunks` sub-ranges and runs them on the rayon
/// thread pool. Returns the merged result (output order is
/// chunk-major, NOT cubiomes' sequential mid-major order — sort
/// the result if order-stability matters).
///
/// Available only when the `parallel` feature is enabled and
/// the target architecture is not `wasm32`.
#[cfg(all(feature = "parallel", not(target_arch = "wasm32")))]
pub fn search_all_48_parallel<F>(
    range: core::ops::RangeInclusive<u64>,
    low_bits: &[u64],
    lbitn: u32,
    n_chunks: usize,
    check: F,
) -> Vec<u64>
where
    F: Fn(u64) -> bool + Sync + Send,
{
    use rayon::prelude::*;
    let start = *range.start();
    let end = *range.end();
    if n_chunks <= 1 || end <= start {
        return search_all_48(range, low_bits, lbitn, &check);
    }
    let total = end - start + 1;
    let sub_ranges: Vec<_> = (0..n_chunks)
        .map(|i| {
            let s = start.wrapping_add(total.wrapping_mul(i as u64) / n_chunks as u64);
            let e = start.wrapping_add(total.wrapping_mul((i + 1) as u64) / n_chunks as u64) - 1;
            s..=e
        })
        .collect();
    sub_ranges
        .par_iter()
        .flat_map_iter(|r| search_all_48(r.clone(), low_bits, lbitn, &check).into_iter())
        .collect()
}

/// `searchAll48(range, low_bits, lbitn, check)` — enumerate 48-bit
/// seeds in `range` whose lower `lbitn` bits match one of
/// `low_bits`, invoking `check` for each candidate. Returns the
/// vector of seeds for which `check` returned `true`.
///
/// Bit-exact port of cubiomes' inner `searchAll48Thread` loop
/// (sequential variant — no file I/O, no resumption, no threading).
/// The output order matches cubiomes': for each `mid` (high-bit
/// chunk), iterate `idx = 0..low_bits.len()` of `low_bits`.
///
/// The `parallel` feature gates a multi-threaded variant; this
/// commit ships only the sequential path.
pub fn search_all_48<F: FnMut(u64) -> bool>(
    range: core::ops::RangeInclusive<u64>,
    low_bits: &[u64],
    lbitn: u32,
    mut check: F,
) -> Vec<u64> {
    let mut out = Vec::new();
    let start = *range.start();
    let end = *range.end();
    if low_bits.is_empty() {
        // Cubiomes' "no low-bit filter" path: iterate every seed.
        let mut seed = start;
        loop {
            if check(seed) {
                out.push(seed);
            }
            if seed == end {
                break;
            }
            seed = seed.wrapping_add(1);
        }
        return out;
    }

    let hstep: u64 = 1u64 << lbitn;
    let hmask: u64 = !(hstep - 1);
    let cnt = low_bits.len();

    let mut mid: u64 = start & hmask;
    // Skip ahead to the first lowBits[idx] whose seed = mid | lb
    // lands >= start.
    let mut idx: usize = 0;
    let mut seed = mid | low_bits[idx];
    while seed < start {
        idx += 1;
        if idx >= cnt {
            idx = 0;
            mid = mid.wrapping_add(hstep);
        }
        seed = mid | low_bits[idx];
    }

    while seed <= end {
        if check(seed) {
            out.push(seed);
        }
        idx += 1;
        if idx >= cnt {
            idx = 0;
            mid = mid.wrapping_add(hstep);
        }
        seed = mid | low_bits[idx];
    }

    out
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

    #[test]
    fn search_all_48_enumerates_low_bit_seeds_in_order() {
        // 4-bit stride, low_bits = [5, 11]. Range 0..=63 should yield
        // (mid=0, lb=5)=5, (mid=0, lb=11)=11, (mid=16, lb=5)=21,
        // (mid=16, lb=11)=27, ..., (mid=48, lb=11)=59.
        let seeds = super::search_all_48(0..=63, &[5, 11], 4, |_| true);
        assert_eq!(seeds, vec![5, 11, 21, 27, 37, 43, 53, 59]);
    }

    #[test]
    fn search_all_48_skips_seeds_before_start() {
        // start=20, lb=[5, 11], stride=16. mid=16, lb=5 → 21 (>= 20)
        // mid=16, lb=11 → 27, mid=32, lb=5 → 37, mid=32, lb=11 → 43.
        let seeds = super::search_all_48(20..=43, &[5, 11], 4, |_| true);
        assert_eq!(seeds, vec![21, 27, 37, 43]);
    }

    #[test]
    fn search_all_48_with_check_filter_returns_only_passing() {
        // Same as above but only seeds > 30 pass.
        let seeds = super::search_all_48(0..=63, &[5, 11], 4, |s| s > 30);
        assert_eq!(seeds, vec![37, 43, 53, 59]);
    }

    #[cfg(all(feature = "parallel", not(target_arch = "wasm32")))]
    #[test]
    fn search_all_48_parallel_matches_sequential() {
        // Use a small enough range that we can sort and compare
        // against the sequential variant. The parallel output is
        // chunk-major, so we sort both before comparison.
        let range = 0..=(1u64 << 16) - 1;
        let low_bits = [0x5_u64, 0x11_u64];
        let lbitn = 8;
        let check = |s: u64| s % 7 == 0;
        let mut seq = super::search_all_48(range.clone(), &low_bits, lbitn, check);
        let mut par = super::search_all_48_parallel(range, &low_bits, lbitn, 4, check);
        seq.sort_unstable();
        par.sort_unstable();
        assert_eq!(seq, par);
    }
}
