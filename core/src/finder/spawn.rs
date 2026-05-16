//! `estimateSpawn` — approximate Overworld spawn point.
//!
//! Bit-exact port of cubiomes' `estimateSpawn` from `finders.c`.
//! Three branches:
//!
//! - MC ≤ Beta 1.7 returns `(0, 0)` (cubiomes can't predict the
//!   sand-block search in those versions).
//! - MC B1.8 – 1.17 runs `locate_biome` over a 512-block square
//!   centred on the origin with a version-dependent spawn-biome
//!   mask.
//! - MC 1.18+ runs cubiomes' `findFittestPos` — two-pass spiral
//!   search that minimises a 6-axis climate fitness function.
//!
//! `getSpawn` (the block-level refinement that consults
//! `mapApproxHeight`) lives in a follow-up.

#![allow(clippy::many_single_char_names)]

use crate::biome::{Biome, get_biome_depth_and_scale};
use crate::biomenoise::surface::SurfaceNoise;
use crate::biomenoise::{SAMPLE_NO_BIOME, SAMPLE_NO_DEPTH};
use crate::finder::Pos;
use crate::finder::locate_biome::locate_biome;
use crate::generator::{Generator, map_approx_height};
use crate::mc_version::{Dimension, MCVersion};
use crate::rng::JavaRng;

const PI: f64 = std::f64::consts::PI;

// B1.8 – 1.0 spawn biome mask (forest, swamp, taiga).
const SPAWN_BIOMES_B10: u64 = (1u64 << 4) | (1u64 << 6) | (1u64 << 5);
// 1.1 – 1.17 spawn biome mask (forest, plains, taiga, taiga_hills,
// wooded_hills, jungle, jungle_hills).
const SPAWN_BIOMES_17: u64 = (1u64 << 4)
    | (1u64 << 1)
    | (1u64 << 5)
    | (1u64 << 19)
    | (1u64 << 18)
    | (1u64 << 21)
    | (1u64 << 22);

/// `estimateSpawn(g, rng)` — return the approximate spawn point.
/// If `rng_out` is `Some`, the post-call `JavaRng` state is written
/// to it (cubiomes uses this to seed `getSpawn`'s refinement).
#[must_use]
pub fn estimate_spawn(g: &Generator, rng_out: Option<&mut JavaRng>) -> Pos {
    if g.mc.is_before(MCVersion::B1_8) {
        return Pos { x: 0, z: 0 };
    }
    if g.mc.is_before(MCVersion::V1_18) {
        let spawn_biomes = if g.mc.is_before(MCVersion::V1_1) {
            SPAWN_BIOMES_B10
        } else {
            SPAWN_BIOMES_17
        };
        let mut rng = JavaRng::new(g.seed);
        let (pos, found) = locate_biome(g, 0, 63, 0, 256, spawn_biomes, 0, &mut rng);
        let spawn = if found > 0 { pos } else { Pos { x: 8, z: 8 } };
        if let Some(out) = rng_out {
            *out = rng;
        }
        return spawn;
    }
    find_fittest_pos(g)
}

fn find_fittest_pos(g: &Generator) -> Pos {
    let mut spawn = Pos { x: 0, z: 0 };
    let mut fitness = calc_fitness(g, 0, 0);
    find_fittest(g, &mut spawn, &mut fitness, 2048.0, 512.0);
    find_fittest(g, &mut spawn, &mut fitness, 512.0, 32.0);
    // centre of chunk
    spawn.x = (spawn.x & !15) + 8;
    spawn.z = (spawn.z & !15) + 8;
    spawn
}

fn find_fittest(g: &Generator, pos: &mut Pos, fitness: &mut u64, maxrad: f64, step: f64) {
    let p = *pos;
    let mut rad = step;
    while rad <= maxrad {
        let mut ang = 0.0_f64;
        let ang_step = step / rad;
        while ang <= PI * 2.0 {
            let x = p.x + (ang.sin() * rad) as i32;
            let z = p.z + (ang.cos() * rad) as i32;
            let fit = calc_fitness(g, x, z);
            if fit < *fitness {
                pos.x = x;
                pos.z = z;
                *fitness = fit;
            }
            ang += ang_step;
        }
        rad += step;
    }
}

/// (lower, upper) climate target ranges. Index 6 is the
/// alternative weirdness range — cubiomes picks the closer of
/// [5] and [6].
const SPAWN_NP: [[i64; 2]; 7] = [
    [-10000, 10000],
    [-10000, 10000],
    [-1100, 10000],
    [-10000, 10000],
    [0, 0],
    [-10000, -1600],
    [1600, 10000],
];

/// Cubiomes' static `calcFitness`. Distance-squared from the
/// configured spawn climate target plus a distance-from-origin
/// penalty.
fn calc_fitness(g: &Generator, x: i32, z: i32) -> u64 {
    let bn = g
        .biome_noise
        .as_ref()
        .expect("Modern OW must be apply_seed'd before calc_fitness");
    let flags = SAMPLE_NO_DEPTH | SAMPLE_NO_BIOME;
    // cubiomes samples at (x>>2, 0, z>>2).
    let (_, np) = bn.sample(x >> 2, 0, z >> 2, flags);

    let mut ds: u64 = 0;
    for i in 0..5 {
        let a = (np[i] as u64).wrapping_sub(SPAWN_NP[i][1] as u64);
        let b = (SPAWN_NP[i][0] as u64).wrapping_sub(np[i] as u64);
        let q = if (a as i64) > 0 {
            a
        } else if (b as i64) > 0 {
            b
        } else {
            0
        };
        ds = ds.wrapping_add(q.wrapping_mul(q));
    }

    let weirdness_part = |range: [i64; 2]| -> u64 {
        let a = (np[5] as u64).wrapping_sub(range[1] as u64);
        let b = (range[0] as u64).wrapping_sub(np[5] as u64);
        let q = if (a as i64) > 0 {
            a
        } else if (b as i64) > 0 {
            b
        } else {
            0
        };
        ds.wrapping_add(q.wrapping_mul(q))
    };
    let ds1 = weirdness_part(SPAWN_NP[5]);
    let ds2 = weirdness_part(SPAWN_NP[6]);
    let ds = ds1.min(ds2);

    let a = (x as i64) * (x as i64);
    let b = (z as i64) * (z as i64);
    if g.mc.is_before(MCVersion::V1_21_3) {
        // mc <= 1.21.1: combine ds with a quartic distance penalty.
        let s = ((a + b) as f64) / (2500.0 * 2500.0);
        let penalty = (s * s * 1.0e8) as u64;
        penalty.wrapping_add(ds)
    } else {
        // mc 1.21.2+: linear distance penalty, ds scaled by 2048².
        ds.wrapping_mul((2048_i64 * 2048) as u64)
            .wrapping_add(a as u64)
            .wrapping_add(b as u64)
    }
}

/// `getSpawn(g)` — block-level spawn refinement. Mirrors cubiomes'
/// `getSpawn` from `finders.c`.
///
/// Starts from [`estimate_spawn`] and then runs a per-MC-version
/// refinement loop:
///
/// - MC ≤ Beta 1.7: returns the estimate directly (no refinement).
/// - MC ≤ 1.12: 1000-iteration random walk, accepting the first
///   position whose biome has `grass > 0` and `mapApproxHeight ≥ grass`.
/// - MC 1.13–1.17: spiral search over a 33×33 chunk window centred
///   on the estimate; for each chunk, scan the 16 cells of its
///   `mapApproxHeight` grid for a grass-eligible cell.
/// - MC ≥ 1.18: 11×11 chunk spiral; per cell, accept if
///   `y > 63` OR biome is frozen ocean / deep frozen ocean / frozen
///   river. Falls back to the chunk centre if no match.
///
/// # Example
///
/// ```
/// use cubioxides::finder::get_spawn;
/// use cubioxides::{Dimension, Generator, MCVersion};
///
/// // Block-level spawn for an arbitrary 1.17 world. The Generator
/// // must be `apply_seed`'d before calling — `get_spawn` does not
/// // re-seed it.
/// let mut g = Generator::new(MCVersion::V1_17, 0);
/// g.apply_seed(Dimension::Overworld, 0xdead_beef);
/// let _spawn = get_spawn(&g);
/// ```
#[must_use]
pub fn get_spawn(g: &Generator) -> Pos {
    let mut rng = JavaRng::new(0);
    let mut spawn = estimate_spawn(g, Some(&mut rng));

    if g.mc.is_before(MCVersion::B1_8) {
        return spawn;
    }

    let sn = SurfaceNoise::init(Dimension::Overworld, g.seed);

    if g.mc.is_before(MCVersion::V1_13) {
        // 1.0 – 1.12: 1000-iter random walk.
        for _ in 0..1000_i32 {
            let mut y = [0.0_f32; 1];
            let mut ids = [Biome::default(); 1];
            map_approx_height(
                &mut y,
                Some(&mut ids),
                g,
                &sn,
                spawn.x >> 2,
                spawn.z >> 2,
                1,
                1,
            );
            let grass = get_biome_depth_and_scale(ids[0].0).map_or(0, |v| v.grass);
            if grass > 0 && y[0] >= grass as f32 {
                break;
            }
            spawn.x += rng.next_int(64) - rng.next_int(64);
            spawn.z += rng.next_int(64) - rng.next_int(64);
        }
        return spawn;
    }

    if g.mc.is_before(MCVersion::V1_18) {
        // 1.13 – 1.17: spiral search over 33×33 chunk window.
        spiral_spawn_search(
            &mut spawn, g, &sn, 1024, -16, 16, true, // 4×4 chunk scan
        );
        return spawn;
    }

    // 1.18+: 11×11 chunk spiral, smaller per-chunk scan.
    spiral_spawn_search(&mut spawn, g, &sn, 121, -5, 5, false);
    spawn
}

/// Cubiomes' spiral-search common loop shared by the 1.13-1.17 and
/// 1.18+ branches of `getSpawn`. The two branches differ in iter
/// count, chunk-window radius, per-chunk accept criterion, and
/// whether they sample the full 4×4 grid (1.17-) or block-by-block
/// (1.18+).
#[allow(clippy::too_many_arguments)]
fn spiral_spawn_search(
    spawn: &mut Pos,
    g: &Generator,
    sn: &SurfaceNoise,
    iters: i32,
    lo: i32,
    hi: i32,
    legacy_117: bool,
) {
    let mut j = 0_i32;
    let mut k = 0_i32;
    let mut u = 0_i32;
    let mut v = -1_i32;
    for _ in 0..iters {
        let in_window = if legacy_117 {
            j > lo && j <= hi && k > lo && k <= hi
        } else {
            j >= lo && j <= hi && k >= lo && k <= hi
        };
        if in_window {
            let cx0 = (spawn.x & !15) + j * 16;
            let cz0 = (spawn.z & !15) + k * 16;
            if legacy_117 {
                // Sample 4×4 grid of biome cells at scale 4.
                let mut y = [0.0_f32; 16];
                let mut ids = [Biome::default(); 16];
                map_approx_height(&mut y, Some(&mut ids), g, sn, cx0 >> 2, cz0 >> 2, 4, 4);
                for ii in 0..4_i32 {
                    for jj in 0..4_i32 {
                        let idx = (jj * 4 + ii) as usize;
                        let grass = get_biome_depth_and_scale(ids[idx].0).map_or(0, |v| v.grass);
                        if grass <= 0 || y[idx] < grass as f32 {
                            continue;
                        }
                        spawn.x = cx0 + ii * 4;
                        spawn.z = cz0 + jj * 4;
                        return;
                    }
                }
            } else {
                // 1.18+: sample 1×1 per cell at scale 4.
                for ii in 0..4_i32 {
                    for jj in 0..4_i32 {
                        let x = cx0 + ii * 4;
                        let z = cz0 + jj * 4;
                        let mut y_buf = [0.0_f32; 1];
                        let mut id_buf = [Biome::default(); 1];
                        map_approx_height(
                            &mut y_buf,
                            Some(&mut id_buf),
                            g,
                            sn,
                            x >> 2,
                            z >> 2,
                            1,
                            1,
                        );
                        let id = id_buf[0].0;
                        // frozen_ocean = 10, deep_frozen_ocean = 50, frozen_river = 11
                        if y_buf[0] > 63.0 || id == 10 || id == 50 || id == 11 {
                            spawn.x = x;
                            spawn.z = z;
                            return;
                        }
                    }
                }
            }
        }
        if j == k || (j < 0 && j == -k) || (j > 0 && j == 1 - k) {
            let tmp = u;
            u = -v;
            v = tmp;
        }
        j += u;
        k += v;
    }
    // No match: snap to chunk centre.
    spawn.x = (spawn.x & !15) + 8;
    spawn.z = (spawn.z & !15) + 8;
}
