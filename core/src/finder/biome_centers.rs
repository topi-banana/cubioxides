//! `getBiomeCenters` — locate connected regions of a biome and
//! return their centroids.
//!
//! Bit-exact port of cubiomes' `getBiomeCenters` (`finders.c`),
//! 1.18+ branch only. The pre-1.18 path relies on the full
//! `checkForBiomes` filter pipeline which we haven't ported yet.

#![allow(
    clippy::too_many_arguments,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use crate::biomenoise::biome_noise::{
    NP_CONTINENTALNESS, NP_EROSION, NP_HUMIDITY, NP_TEMPERATURE, NP_WEIRDNESS,
};
use crate::finder::Pos;
use crate::finder::biome_para::get_biome_para_limits;
use crate::generator::{Generator, Range};
use crate::mc_version::{Dimension, MCVersion};

/// Result of [`get_biome_centers`]. Returns the discovered centres
/// and (per centre) the connected-region area in cells.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BiomeCenters {
    /// Located region centres (in block coordinates, post-`r.scale` scaling).
    pub pos: Vec<Pos>,
    /// Per-centre region size in scaled cells.
    pub sizes: Vec<i32>,
}

/// `getBiomeCenters(g, r, match_id, minsiz, tol, nmax)` —
/// locate every connected region of biome `match_id` in the
/// `r`-area whose cell count is at least `minsiz`, returning their
/// centroids and sizes (up to `nmax` matches).
///
/// Supports any MC version where [`Generator::gen_biomes`] works
/// (Beta + Layered 1.7-1.17 use a tile-scan + flood-fill that
/// mirrors cubiomes' pre-1.18 path; 1.18+ uses the climate-axis
/// pre-filter + flood-fill from cubiomes' Modern path). Returns
/// `None` if there's no `BiomeNoise` populated when 1.18+ needs
/// one.
pub fn get_biome_centers(
    g: &mut Generator,
    r: Range,
    match_id: i32,
    minsiz: i32,
    tol: i32,
    nmax: usize,
) -> Option<BiomeCenters> {
    let minsiz = if minsiz <= 0 { 1 } else { minsiz };
    let tol_used = if tol <= 0 { 1 } else { tol };
    let sx = r.sx as i32;
    let sz = r.sz as i32;
    let mut ids: Vec<i32> = vec![-1; (sx as usize) * (sz as usize)];
    let mut step = tol_used;
    if tol == 1 {
        // cubiomes: step = 1 + floor(sqrt(minsiz) * 0.5)
        step = 1 + ((f64::from(minsiz).sqrt() * 0.5).floor() as i32);
    }

    let effective_match;
    if g.mc.is_at_least(MCVersion::V1_18) {
        // 1.18+ path — climate-axis pre-filter.
        let lim = get_biome_para_limits(g.mc, match_id);
        let para: [usize; 5] = [
            NP_TEMPERATURE,
            NP_HUMIDITY,
            NP_EROSION,
            NP_CONTINENTALNESS,
            NP_WEIRDNESS,
        ];
        if let Some(lim) = lim {
            let bn = g
                .biome_noise
                .as_ref()
                .expect("BiomeNoise must be seeded for 1.18+ biome centers");
            let mut j = 0_i32;
            while j < sz {
                let mut i = 0_i32;
                while i < sx {
                    for &p in &para {
                        let (plo, phi) = lim[p];
                        if plo == i32::MIN && phi == i32::MAX {
                            continue;
                        }
                        let dpn = &bn.climate[p];
                        let px = f64::from(r.x + i) * f64::from(r.scale) / 4.0;
                        let pz = f64::from(r.z + j) * f64::from(r.scale) / 4.0;
                        let v = (10000.0 * dpn.sample(px, 0.0, pz)) as i32;
                        if v < plo || v > phi {
                            ids[(j as usize) * (sx as usize) + (i as usize)] = -2;
                            break;
                        }
                    }
                    i += step;
                }
                j += step;
            }
        }
        effective_match = -1;
    } else {
        // Pre-1.18 path — tile-scan via single-biome `check_for_biomes`.
        scan_pre_118(g, &r, &mut ids, sx, sz, match_id);
        effective_match = match_id;
    }

    g.apply_seed(Dimension::Overworld, g.seed);

    let mut centers = BiomeCenters::default();
    let mut j = 0_i32;
    'outer: while j < sz {
        let mut i = 0_i32;
        while i < sx {
            let idx = (j as usize) * (sx as usize) + (i as usize);
            if ids[idx] == effective_match {
                if let Some((center, area)) =
                    flood_fill_gen(g, &mut ids, sx, sz, &r, match_id, tol_used, i, j)
                {
                    if area >= minsiz {
                        centers.pos.push(center);
                        centers.sizes.push(area);
                        if centers.pos.len() >= nmax {
                            break 'outer;
                        }
                    }
                }
            }
            i += step;
        }
        j += step;
    }
    Some(centers)
}

/// Pre-1.18 tile scan — for each `ts × ts` tile in the area, run
/// `gen_biomes` once and, if any cell matches `match_id`, copy the
/// tile's biome ids into the global `ids` buffer. Cells in tiles
/// that don't contain `match_id` stay at `-1`, which the outer
/// flood-fill's `effective_match = match_id` check naturally
/// rejects. Mirrors cubiomes' `getBiomeCenters` pre-1.18 branch.
fn scan_pre_118(g: &mut Generator, r: &Range, ids: &mut [i32], sx: i32, sz: i32, match_id: i32) {
    use crate::biome::Biome;
    let ts: i32 = {
        let mut t = 32 / r.scale.max(1);
        if (r.sx as i32) + (r.sz as i32) < 32 {
            t = 8;
        }
        t.max(1)
    };
    let tx = (f64::from(r.x) / f64::from(ts)).floor() as i32;
    let tz = (f64::from(r.z) / f64::from(ts)).floor() as i32;
    let tw = (f64::from(r.x + sx) / f64::from(ts)).ceil() as i32 - tx;
    let th = (f64::from(r.z + sz) / f64::from(ts)).ceil() as i32 - tz;
    let mut tile = vec![Biome(0); (ts as usize) * (ts as usize)];
    for tj in 0..th {
        for ti in 0..tw {
            let tr = Range {
                scale: r.scale,
                x: (tx + ti) * ts,
                z: (tz + tj) * ts,
                sx: ts as u32,
                sz: ts as u32,
                y: 0,
                sy: 1,
            };
            g.gen_biomes(&mut tile, tr);
            // Single-biome filter: any cell == match_id means pass.
            let mut pass = false;
            for cell in &tile {
                if cell.0 == match_id {
                    pass = true;
                    break;
                }
            }
            if !pass {
                continue;
            }
            for j in 0..ts {
                let jj = tr.z + j - r.z;
                if jj < 0 || jj >= sz {
                    continue;
                }
                for i in 0..ts {
                    let ii = tr.x + i - r.x;
                    if ii < 0 || ii >= sx {
                        continue;
                    }
                    ids[(jj as usize) * (sx as usize) + (ii as usize)] =
                        tile[(j as usize) * (ts as usize) + (i as usize)].0;
                }
            }
        }
    }
}

#[allow(clippy::cast_lossless)]
fn flood_fill_gen(
    g: &Generator,
    ids: &mut [i32],
    sx: i32,
    sz: i32,
    r: &Range,
    match_id: i32,
    tol: i32,
    i0: i32,
    j0: i32,
) -> Option<(Pos, i32)> {
    let mut queue: Vec<(i32, i32, i32)> = vec![(i0, j0, 0)];
    let mut sum_x: i64 = 0;
    let mut sum_z: i64 = 0;
    let mut n: i32 = 0;
    while let Some((mut i, mut j, mut d)) = queue.pop() {
        let k = (j as usize) * (sx as usize) + (i as usize);
        let id_at = ids[k];
        if id_at == i32::MAX {
            continue;
        }
        ids[k] = i32::MAX;
        let x = r.x + i;
        let z = r.z + j;
        let sampled = g.biome_at(r.scale, x, r.y, z).0;
        if sampled == match_id {
            sum_x += x as i64;
            sum_z += z as i64;
            n += 1;
            d = 0;
        } else {
            d += 1;
            if d >= tol {
                continue;
            }
        }
        let next = [(i, j - 1, d), (i, j + 1, d), (i - 1, j, d), (i + 1, j, d)];
        for (next_i, next_j, next_d) in next {
            i = next_i;
            j = next_j;
            if i < 0 || i >= sx || j < 0 || j >= sz {
                continue;
            }
            let kk = (j as usize) * (sx as usize) + (i as usize);
            if ids[kk] == i32::MAX {
                continue;
            }
            queue.push((next_i, next_j, next_d));
        }
    }
    if n == 0 {
        return None;
    }
    let center_x = ((sum_x as f64 / f64::from(n) + 0.5) * f64::from(r.scale)).round() as i32;
    let center_z = ((sum_z as f64 / f64::from(n) + 0.5) * f64::from(r.scale)).round() as i32;
    Some((
        Pos {
            x: center_x,
            z: center_z,
        },
        n,
    ))
}
