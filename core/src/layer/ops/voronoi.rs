//! Voronoi 1:1 access layers.
//!
//! Two variants live here:
//!
//! - [`map_voronoi`] / [`map_voronoi_plane`] for MC 1.0-1.14, seeded
//!   by the truncated SHA-256 digest from [`crate::sha::voronoi_sha`]
//!   and queried via [`voronoi_access_3d`].
//! - [`map_voronoi114`] for MC 1.15+, seeded directly by chunk seeds.
//!
//! Both consume a 1:4-scale parent grid and emit a 1:1 window. The
//! pre-1.15 path also supports 3D output via [`map_voronoi_plane`]
//! (a y argument selects the slice).

#![allow(clippy::many_single_char_names, clippy::too_many_arguments)]

use crate::biome::Biome;
use crate::rng::{get_chunk_seed, mc_first_int, mc_step_seed};

/// `mapVoronoi114` — parent at scale 1:4, output at scale 1:1.
///
/// `parent` must be sized `parent_w * parent_h`. Pixel coordinates
/// in `(x, z, w, h)` are in the *output* (1:1) frame; the layer
/// downscales by 4 internally.
pub fn map_voronoi114(
    start_salt: u64,
    start_seed: u64,
    parent: &[Biome],
    parent_x: i32,
    parent_z: i32,
    parent_w: usize,
    parent_h: usize,
    out: &mut [Biome],
    x: i32,
    z: i32,
    w: usize,
    h: usize,
) {
    // cubiomes shifts the output frame by -2 before the divide-by-4.
    let x = x - 2;
    let z = z - 2;

    assert!(
        parent.len() >= parent_w * parent_h,
        "map_voronoi114: parent slice too small"
    );
    assert!(out.len() >= w * h, "map_voronoi114: output slice too small");

    let mut buf = vec![Biome::NONE; w * h];
    let st = start_salt;
    let ss = start_seed;

    if parent_h == 0 || parent_w == 0 {
        return;
    }

    for pj in 0..(parent_h - 1) {
        let mut v00 = parent[pj * parent_w].id();
        let mut v01 = parent[(pj + 1) * parent_w].id();
        let pjz = parent_z + pj as i32;
        let j4 = pjz * 4 - z;

        for pi in 0..(parent_w - 1) {
            let pix = parent_x + pi as i32;
            let i4 = pix * 4 - x;

            let v10 = parent[(pi + 1) + pj * parent_w].id();
            let v11 = parent[(pi + 1) + (pj + 1) * parent_w].id();

            if v00 == v01 && v00 == v10 && v00 == v11 {
                fill_4x4(&mut buf, w, h, i4, j4, v00);
                v00 = v10;
                v01 = v11;
                continue;
            }

            // Four chunk-seed-driven corner offsets, each generating
            // a 2-component perturbation (i.e. (di, dj) in 1/10240
            // sub-cell units, scaled by 36).
            let mut cs = get_chunk_seed(ss, (pi as i32 + parent_x) * 4, (pj as i32 + parent_z) * 4);
            let da1 = (i64::from(mc_first_int(cs, 1024)) - 512) * 36;
            cs = mc_step_seed(cs, st);
            let da2 = (i64::from(mc_first_int(cs, 1024)) - 512) * 36;

            cs = get_chunk_seed(
                ss,
                (pi as i32 + parent_x + 1) * 4,
                (pj as i32 + parent_z) * 4,
            );
            let db1 = (i64::from(mc_first_int(cs, 1024)) - 512) * 36 + 40 * 1024;
            cs = mc_step_seed(cs, st);
            let db2 = (i64::from(mc_first_int(cs, 1024)) - 512) * 36;

            cs = get_chunk_seed(
                ss,
                (pi as i32 + parent_x) * 4,
                (pj as i32 + parent_z + 1) * 4,
            );
            let dc1 = (i64::from(mc_first_int(cs, 1024)) - 512) * 36;
            cs = mc_step_seed(cs, st);
            let dc2 = (i64::from(mc_first_int(cs, 1024)) - 512) * 36 + 40 * 1024;

            cs = get_chunk_seed(
                ss,
                (pi as i32 + parent_x + 1) * 4,
                (pj as i32 + parent_z + 1) * 4,
            );
            let dd1 = (i64::from(mc_first_int(cs, 1024)) - 512) * 36 + 40 * 1024;
            cs = mc_step_seed(cs, st);
            let dd2 = (i64::from(mc_first_int(cs, 1024)) - 512) * 36 + 40 * 1024;

            for jj in 0..4i64 {
                let j = j4 + jj as i32;
                if j < 0 || j >= h as i32 {
                    continue;
                }

                let mj = 10240 * jj;
                let sja = (mj - da2) * (mj - da2);
                let sjb = (mj - db2) * (mj - db2);
                let sjc = (mj - dc2) * (mj - dc2);
                let sjd = (mj - dd2) * (mj - dd2);

                for ii in 0..4i64 {
                    let i = i4 + ii as i32;
                    if i < 0 || i >= w as i32 {
                        continue;
                    }

                    let mi = 10240 * ii;
                    let da = (mi - da1) * (mi - da1) + sja;
                    let db = (mi - db1) * (mi - db1) + sjb;
                    let dc = (mi - dc1) * (mi - dc1) + sjc;
                    let dd = (mi - dd1) * (mi - dd1) + sjd;

                    let v = if da < db && da < dc && da < dd {
                        v00
                    } else if db < da && db < dc && db < dd {
                        v10
                    } else if dc < da && dc < db && dc < dd {
                        v01
                    } else {
                        v11
                    };

                    buf[j as usize * w + i as usize] = Biome(v);
                }
            }

            v00 = v10;
            v01 = v11;
        }
    }

    out[..w * h].copy_from_slice(&buf);
}

fn fill_4x4(buf: &mut [Biome], w: usize, h: usize, i4: i32, j4: i32, value: i32) {
    for jj in 0..4 {
        let j = j4 + jj;
        if j < 0 || j >= h as i32 {
            continue;
        }
        for ii in 0..4 {
            let i = i4 + ii;
            if i < 0 || i >= w as i32 {
                continue;
            }
            buf[j as usize * w + i as usize] = Biome(value);
        }
    }
}

/// Pre-1.15 SHA-driven Voronoi cell: hash `(a, b, c)` through six
/// rounds of [`mc_step_seed`] then read three 10-bit fields shifted
/// to `[-512, 512)` and scaled by 36. Returns `(rx, ry, rz)` — the
/// 1:1-unit offset of the seed point inside the 4-block cell at
/// `(a*4, b*4, c*4)`. Internal helper for [`map_voronoi_plane`] and
/// [`voronoi_access_3d`].
#[inline]
fn voronoi_cell(sha: u64, a: i32, b: i32, c: i32) -> (i32, i32, i32) {
    let mut s = sha;
    s = mc_step_seed(s, a as u64);
    s = mc_step_seed(s, b as u64);
    s = mc_step_seed(s, c as u64);
    s = mc_step_seed(s, a as u64);
    s = mc_step_seed(s, b as u64);
    s = mc_step_seed(s, c as u64);
    let rx = ((((s >> 24) & 1023) as i32) - 512) * 36;
    s = mc_step_seed(s, sha);
    let ry = ((((s >> 24) & 1023) as i32) - 512) * 36;
    s = mc_step_seed(s, sha);
    let rz = ((((s >> 24) & 1023) as i32) - 512) * 36;
    (rx, ry, rz)
}

/// `voronoiAccess3D` — invert the Voronoi 1:1 mapping for a single
/// `(x, y, z)` block, returning the `(x4, y4, z4)` 1:4-scale cell that
/// owns it. Used by 1.15+ Nether / End sampling and any code that
/// needs to map a 1:1 query back to the underlying biome grid.
#[must_use]
pub fn voronoi_access_3d(sha: u64, x: i32, y: i32, z: i32) -> (i32, i32, i32) {
    let x = x - 2;
    let y = y - 2;
    let z = z - 2;
    let p_x = x >> 2;
    let p_y = y >> 2;
    let p_z = z >> 2;
    let dx = (x & 3) * 10240;
    let dy = (y & 3) * 10240;
    let dz = (z & 3) * 10240;

    let mut best = (p_x, p_y, p_z);
    let mut dmin: u64 = u64::MAX;
    for i in 0..8 {
        let bx = i32::from((i & 4) != 0);
        let by = i32::from((i & 2) != 0);
        let bz = i32::from((i & 1) != 0);
        let cx = p_x + bx;
        let cy = p_y + by;
        let cz = p_z + bz;
        let (rx, ry, rz) = voronoi_cell(sha, cx, cy, cz);
        let rx = rx + dx - 40 * 1024 * bx;
        let ry = ry + dy - 40 * 1024 * by;
        let rz = rz + dz - 40 * 1024 * bz;
        let d = (rx as i64 * rx as i64 + ry as i64 * ry as i64 + rz as i64 * rz as i64) as u64;
        if d < dmin {
            dmin = d;
            best = (cx, cy, cz);
        }
    }
    best
}

/// `mapVoronoiPlane` — sample a 1:1 plane through the SHA-driven
/// Voronoi field at a fixed `y` (in the 1:1 frame). The 1:4 parent
/// grid covers `(parent_x, parent_z, parent_w, parent_h)` and must be
/// large enough for a `2x2` window around every output cell — see
/// the [`map_voronoi`] wrapper for the canonical rectangle.
#[allow(clippy::too_many_lines)]
pub fn map_voronoi_plane(
    sha: u64,
    parent: &[Biome],
    parent_x: i32,
    parent_z: i32,
    parent_w: usize,
    parent_h: usize,
    out: &mut [Biome],
    x: i32,
    y: i32,
    z: i32,
    w: usize,
    h: usize,
) {
    const A: i32 = 40 * 1024;
    const B: i32 = 20 * 1024;

    assert!(
        parent.len() >= parent_w * parent_h,
        "map_voronoi_plane: parent slice too small"
    );
    assert!(
        out.len() >= w * h,
        "map_voronoi_plane: output slice too small"
    );

    let x = x - 2;
    let y = y - 2;
    let z = z - 2;

    if parent_h == 0 || parent_w == 0 {
        return;
    }

    for pj in 0..parent_h.saturating_sub(1) {
        let mut v00 = parent[pj * parent_w].id();
        let mut v10 = parent[(pj + 1) * parent_w].id();
        let pjz = parent_z + pj as i32;
        let j4 = pjz * 4 - z;

        let mut prev_skip = true;
        // First-iteration corner cache (only valid after prev_skip falls).
        let mut x000 = 0i32;
        let mut y000 = 0i32;
        let mut z000 = 0i32;
        let mut x001 = 0i32;
        let mut y001 = 0i32;
        let mut z001 = 0i32;
        let mut x100 = 0i32;
        let mut y100 = 0i32;
        let mut z100 = 0i32;
        let mut x101 = 0i32;
        let mut y101 = 0i32;
        let mut z101 = 0i32;

        for pi in 0..parent_w.saturating_sub(1) {
            let v01 = parent[pj * parent_w + pi + 1].id();
            let v11 = parent[(pj + 1) * parent_w + pi + 1].id();
            let pix = parent_x + pi as i32;
            let i4 = pix * 4 - x;

            if v00 == v01 && v00 == v10 && v00 == v11 {
                fill_4x4(out, w, h, i4, j4, v00);
                prev_skip = true;
                v00 = v01;
                v10 = v11;
                continue;
            }
            if prev_skip {
                let (rx, ry, rz) = voronoi_cell(sha, pix, y - 1, pjz);
                x000 = rx;
                y000 = ry;
                z000 = rz;
                let (rx, ry, rz) = voronoi_cell(sha, pix, y, pjz);
                x001 = rx;
                y001 = ry;
                z001 = rz;
                let (rx, ry, rz) = voronoi_cell(sha, pix, y - 1, pjz + 1);
                x100 = rx;
                y100 = ry;
                z100 = rz;
                let (rx, ry, rz) = voronoi_cell(sha, pix, y, pjz + 1);
                x101 = rx;
                y101 = ry;
                z101 = rz;
                prev_skip = false;
            }
            let (x010, y010, z010) = voronoi_cell(sha, pix + 1, y - 1, pjz);
            let (x011, y011, z011) = voronoi_cell(sha, pix + 1, y, pjz);
            let (x110, y110, z110) = voronoi_cell(sha, pix + 1, y - 1, pjz + 1);
            let (x111, y111, z111) = voronoi_cell(sha, pix + 1, y, pjz + 1);

            for jj in 0..4i32 {
                let j = j4 + jj;
                if j < 0 || j >= h as i32 {
                    continue;
                }
                for ii in 0..4i32 {
                    let i = i4 + ii;
                    if i < 0 || i >= w as i32 {
                        continue;
                    }
                    let dx = ii * 10 * 1024;
                    let dz = jj * 10 * 1024;
                    let mut dmin = u64::MAX;
                    let mut v = v00;

                    let mut consider = |rx: i32, ry: i32, rz: i32, candidate: i32| {
                        let r0 = rx as i64;
                        let r1 = ry as i64;
                        let r2 = rz as i64;
                        let d = (r0 * r0 + r1 * r1 + r2 * r2) as u64;
                        if d < dmin {
                            dmin = d;
                            v = candidate;
                        }
                    };

                    consider(x000 + dx, y000 + B, z000 + dz, v00);
                    consider(x001 + dx, y001 - B, z001 + dz, v00);
                    consider(x010 - A + dx, y010 + B, z010 + dz, v01);
                    consider(x011 - A + dx, y011 - B, z011 + dz, v01);
                    consider(x100 + dx, y100 + B, z100 - A + dz, v10);
                    consider(x101 + dx, y101 - B, z101 - A + dz, v10);
                    consider(x110 - A + dx, y110 + B, z110 - A + dz, v11);
                    consider(x111 - A + dx, y111 - B, z111 - A + dz, v11);

                    out[j as usize * w + i as usize] = Biome(v);
                }
            }

            // Shift right edge to left for next pi.
            x000 = x010;
            y000 = y010;
            z000 = z010;
            x100 = x110;
            y100 = y110;
            z100 = z110;
            x001 = x011;
            y001 = y011;
            z001 = z011;
            x101 = x111;
            y101 = y111;
            z101 = z111;
            v00 = v01;
            v10 = v11;
        }
    }
}

/// `mapVoronoi` — pre-1.15 layer entry point. Same coordinate
/// conventions as [`map_voronoi114`] (caller computes `(parent_x,
/// parent_z, parent_w, parent_h)` from `(x, z, w, h)` and prepopulates
/// the parent grid). Internally shifts `(x, z)` by -2 and delegates
/// to [`map_voronoi_plane`] with `y = 0` — matching cubiomes' double
/// shift.
pub fn map_voronoi(
    sha: u64,
    parent: &[Biome],
    parent_x: i32,
    parent_z: i32,
    parent_w: usize,
    parent_h: usize,
    out: &mut [Biome],
    x: i32,
    z: i32,
    w: usize,
    h: usize,
) {
    map_voronoi_plane(
        sha,
        parent,
        parent_x,
        parent_z,
        parent_w,
        parent_h,
        out,
        x - 2,
        0,
        z - 2,
        w,
        h,
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Compute the `(parent_x, parent_z, parent_w, parent_h)` cubiomes
    /// expects for an output `(x, z, w, h)` (mapVoronoi114).
    fn parent_rect(x: i32, z: i32, w: usize, h: usize) -> (i32, i32, usize, usize) {
        let x = x - 2;
        let z = z - 2;
        let p_x = x >> 2;
        let p_z = z >> 2;
        let p_w = (((x + w as i32) >> 2) - p_x + 2) as usize;
        let p_h = (((z + h as i32) >> 2) - p_z + 2) as usize;
        (p_x, p_z, p_w, p_h)
    }

    /// Cubiomes mapVoronoi (1.0-1.14) shifts (x, z) twice — once in
    /// mapVoronoi itself, then again inside mapVoronoiPlane.
    fn parent_rect_pre_115(x: i32, z: i32, w: usize, h: usize) -> (i32, i32, usize, usize) {
        let x = x - 2;
        let z = z - 2;
        let p_x = x >> 2;
        let p_z = z >> 2;
        let p_w = (((x + w as i32) >> 2) - p_x + 2) as usize;
        let p_h = (((z + h as i32) >> 2) - p_z + 2) as usize;
        (p_x, p_z, p_w, p_h)
    }

    #[test]
    fn uniform_parent_yields_uniform_output() {
        let (px, pz, pw, ph) = parent_rect(0, 0, 8, 8);
        let parent = vec![Biome::FOREST; pw * ph];
        let mut out = vec![Biome::NONE; 8 * 8];
        map_voronoi114(0, 0, &parent, px, pz, pw, ph, &mut out, 0, 0, 8, 8);
        for cell in &out {
            assert_eq!(*cell, Biome::FOREST);
        }
    }

    #[test]
    fn deterministic_with_distinct_corners() {
        let (px, pz, pw, ph) = parent_rect(0, 0, 8, 8);
        let mut parent = vec![Biome::FOREST; pw * ph];
        // Pick an interior cell of the parent and flip it; the exact
        // index doesn't matter for the determinism check.
        if pw >= 2 && ph >= 2 {
            parent[1 + pw] = Biome::PLAINS;
        }
        let mut a = vec![Biome::NONE; 8 * 8];
        let mut b = vec![Biome::NONE; 8 * 8];
        map_voronoi114(123, 456, &parent, px, pz, pw, ph, &mut a, 0, 0, 8, 8);
        map_voronoi114(123, 456, &parent, px, pz, pw, ph, &mut b, 0, 0, 8, 8);
        assert_eq!(a, b);
    }

    #[test]
    fn uniform_parent_yields_uniform_pre115_output() {
        let (px, pz, pw, ph) = parent_rect_pre_115(0, 0, 8, 8);
        let parent = vec![Biome::FOREST; pw * ph];
        let mut out = vec![Biome::NONE; 8 * 8];
        map_voronoi(0xdead_beef, &parent, px, pz, pw, ph, &mut out, 0, 0, 8, 8);
        for cell in &out {
            assert_eq!(*cell, Biome::FOREST);
        }
    }

    #[test]
    fn voronoi_access_3d_deterministic() {
        for &seed in &[0u64, 1, 0xdead_beef, u64::MAX] {
            for x in [-10i32, 0, 1, 7, 123].iter().copied() {
                for z in [-3i32, 0, 5, 64].iter().copied() {
                    assert_eq!(
                        voronoi_access_3d(seed, x, 0, z),
                        voronoi_access_3d(seed, x, 0, z)
                    );
                }
            }
        }
    }
}
