//! `mapVoronoi114` — 1.15+ Voronoi 1:1 access layer.
//!
//! Bit-exact port of cubiomes' `mapVoronoi114`. Reads a 1:4-scale
//! parent grid and emits a 1:1 window: for each 4x4 cell-of-parent
//! block the four corner biome IDs become Voronoi seed points whose
//! positions are perturbed by a chunk-seed-driven offset.
//!
//! The earlier `mapVoronoi` (1.0-1.14, SHA-256 driven via
//! `getVoronoiCell` / `mapVoronoiPlane`) lands in a follow-up commit
//! once the SHA helper is ported.

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

#[cfg(test)]
mod tests {
    use super::*;

    /// Compute the `(parent_x, parent_z, parent_w, parent_h)` cubiomes
    /// expects for an output `(x, z, w, h)`.
    fn parent_rect(x: i32, z: i32, w: usize, h: usize) -> (i32, i32, usize, usize) {
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
}
