//! `mapApproxHeight` — approximate surface height generator.
//!
//! Bit-exact port of cubiomes' `mapApproxHeight` from `generator.c`.
//! Dispatches on (dim, mc):
//!
//! - **Nether**: returns 127 immediately (cubiomes' int-return
//!   convention; `y` is *not* written).
//! - **End**: MC ≤ 1.8 returns 1 (`y` unwritten); MC ≥ 1.9 delegates
//!   to [`crate::biomenoise::end_surface::map_end_surface_height`]
//!   at scale 4.
//! - **MC ≥ 1.18 Overworld**: samples `BiomeNoise.np[NP_DEPTH]` per
//!   cell, writes `np[NP_DEPTH] / 76.0` into `y`.
//! - **Legacy 1.0–1.17 Overworld**: 5×5 weighted kernel over biome
//!   depth/scale at scale 4, then per-cell octave-depth offset +
//!   binary search for the topmost surface block.
//! - **Beta (B1.0–B1.7)**: per-cell `approx_surface_beta` sampling at
//!   `(cell_x*4 + 2, cell_z*4 + 2)`.

#![allow(
    clippy::many_single_char_names,
    clippy::needless_range_loop,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_lossless,
    clippy::excessive_precision,
    clippy::assign_op_pattern
)]

use crate::biome::{Biome, get_biome_depth_and_scale};
use crate::biomenoise::biome_noise::NP_DEPTH;
use crate::biomenoise::end_surface::map_end_surface_height;
use crate::biomenoise::surface::SurfaceNoise;
use crate::generator::{Generator, Range};
use crate::mc_version::{Dimension, MCVersion};

/// Per-cell weighting kernel for the legacy 1.0-1.17 depth/scale
/// blend. `10 / (sqrt(i*i + j*j) + 0.2)` with `(i, j) ∈ [-2, 2]`.
/// Matches cubiomes' `biome_kernel`.
#[allow(clippy::approx_constant)]
const BIOME_KERNEL: [f32; 25] = [
    3.302_044_127,
    4.104_975_761,
    4.545_454_545,
    4.104_975_761,
    3.302_044_127,
    4.104_975_761,
    6.194_967_155,
    8.333_333_333,
    6.194_967_155,
    4.104_975_761,
    4.545_454_545,
    8.333_333_333,
    50.000_000_000,
    8.333_333_333,
    4.545_454_545,
    4.104_975_761,
    6.194_967_155,
    8.333_333_333,
    6.194_967_155,
    4.104_975_761,
    3.302_044_127,
    4.104_975_761,
    4.545_454_545,
    4.104_975_761,
    3.302_044_127,
];

/// `mapApproxHeight(y, ids, g, sn, x, z, w, h)` — fill `y` (and
/// optionally `ids`) with the approximate surface block-Y for each
/// `(x..x+w, z..z+h)` cell at scale 4 (the native biome-cache scale).
///
/// Returns 0 on success, 127 for the Nether (special convention: the
/// `y` buffer is *not* written), 1 for unsupported End versions
/// (MC ≤ 1.8).
#[allow(clippy::too_many_arguments)]
pub fn map_approx_height(
    y: &mut [f32],
    ids: Option<&mut [Biome]>,
    g: &Generator,
    sn: &SurfaceNoise,
    x: i32,
    z: i32,
    w: i32,
    h: i32,
) -> i32 {
    assert!(
        y.len() >= (w * h) as usize,
        "map_approx_height: y buffer too small ({}x{}={} vs len {})",
        w,
        h,
        w * h,
        y.len()
    );
    let dim = g
        .dim
        .expect("map_approx_height: generator must have apply_seed'd dim");

    if dim == Dimension::Nether {
        return 127;
    }

    if dim == Dimension::End {
        if g.mc.is_before(MCVersion::V1_9) {
            return 1;
        }
        let en = g
            .end
            .as_ref()
            .expect("map_approx_height: End dim must have EndNoise");
        return map_end_surface_height(y, en, sn, x, z, w, h, 4, 0);
    }

    if g.mc.is_at_least(MCVersion::V1_18) {
        let bn = g
            .biome_noise
            .as_ref()
            .expect("map_approx_height: 1.18+ Overworld must have BiomeNoise");
        // cubiomes flags = 0 (let SAMPLE_NO_SHIFT default off).
        let flags = 0_u32;
        if let Some(ids) = ids {
            assert!(
                ids.len() >= (w * h) as usize,
                "map_approx_height: ids buffer too small"
            );
            for j in 0..h {
                for i in 0..w {
                    let (id, np) = bn.sample(x + i, 0, z + j, flags);
                    ids[(j * w + i) as usize] = Biome(id);
                    y[(j * w + i) as usize] = np[NP_DEPTH] as f32 / 76.0;
                }
            }
        } else {
            for j in 0..h {
                for i in 0..w {
                    let (_, np) = bn.sample(x + i, 0, z + j, flags);
                    y[(j * w + i) as usize] = np[NP_DEPTH] as f32 / 76.0;
                }
            }
        }
        return 0;
    }

    if g.mc.is_before(MCVersion::B1_8) {
        // Beta: per-cell `approx_surface_beta` sampling at
        // (cell_x*4+2, cell_z*4+2) block coords. Cubiomes uses a
        // fresh SurfaceNoiseBeta inside mapApproxHeight (not the
        // one cached on the Generator), matching the C code.
        let bnb = g
            .biome_noise_beta
            .as_ref()
            .expect("Beta map_approx_height: Generator must have biome_noise_beta");
        let snb = crate::biomenoise::surface_beta::SurfaceNoiseBeta::init(g.seed);
        for j in 0..h {
            for i in 0..w {
                let sample_x = (x + i) * 4 + 2;
                let sample_z = (z + j) * 4 + 2;
                let h_val = crate::biomenoise::surface_beta::approx_surface_beta(
                    bnb, &snb, sample_x, sample_z,
                );
                y[(j * w + i) as usize] = h_val as f32;
            }
        }
        return 0;
    }

    // 1.0 – 1.17 Overworld: 5×5 weighted kernel for depth/scale,
    // then per-cell octave-depth offset + binary search for the
    // topmost surface block.
    map_approx_height_legacy(y, ids, g, sn, x, z, w, h);
    0
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn map_approx_height_legacy(
    y: &mut [f32],
    ids: Option<&mut [Biome]>,
    g: &Generator,
    sn: &SurfaceNoise,
    x: i32,
    z: i32,
    w: i32,
    h: i32,
) {
    // Sample biomes over a (w+5) × (h+5) region at scale 4, centred
    // on (x-2, z-2). The 5-cell border lets the 5×5 kernel access
    // neighbouring biome cells for every (i, j) in [0, w) × [0, h).
    let r = Range {
        scale: 4,
        x: x - 2,
        z: z - 2,
        sx: (w + 5) as u32,
        sz: (h + 5) as u32,
        y: 0,
        sy: 1,
    };
    let mut cache = vec![Biome::default(); r.cell_count()];
    g.gen_biomes(&mut cache, r);

    let w_u = w as usize;
    let h_u = h as usize;
    let sx = r.sx as usize;
    let mut depth_buf = vec![0.0_f64; w_u * h_u];
    let mut scale_buf = vec![0.0_f64; w_u * h_u];

    // 5×5 kernel reduction → per-cell (depth, scale).
    for j in 0..h_u {
        for i in 0..w_u {
            let id0 = cache[(j + 2) * sx + (i + 2)].0;
            let (d0, _s0) = match get_biome_depth_and_scale(id0) {
                Some(v) => (v.depth, v.scale),
                None => (0.0, 0.0),
            };

            let mut wt: f64 = 0.0;
            let mut ws: f64 = 0.0;
            let mut wd: f64 = 0.0;
            for jj in 0..5_usize {
                for ii in 0..5_usize {
                    let id = cache[(j + jj) * sx + (i + ii)].0;
                    let (d, s) = match get_biome_depth_and_scale(id) {
                        Some(v) => (v.depth, v.scale),
                        None => (0.0, 0.0),
                    };
                    // cubiomes stores `weight` as `float`, so we
                    // narrow the divide result to f32 to match.
                    let mut weight = (BIOME_KERNEL[jj * 5 + ii] as f64 / (d + 2.0)) as f32;
                    if d > d0 {
                        weight *= 0.5;
                    }
                    ws += s * weight as f64;
                    wd += d * weight as f64;
                    wt += weight as f64;
                }
            }
            ws /= wt;
            wd /= wt;
            ws = ws * 0.9 + 0.1;
            wd = (wd * 4.0 - 1.0) / 8.0;
            ws = 96.0 / ws;
            wd *= 17.0 / 64.0;
            depth_buf[j * w_u + i] = wd;
            scale_buf[j * w_u + i] = ws;
        }
    }

    // Write biome ids if requested.
    if let Some(ids) = ids {
        for j in 0..h_u {
            for i in 0..w_u {
                ids[j * w_u + i] = cache[(j + 2) * sx + (i + 2)];
            }
        }
    }

    // Per-cell octave-depth offset + binary search over y.
    let oct_depth = sn
        .oct_depth
        .as_ref()
        .expect("legacy mapApproxHeight needs SurfaceNoise::oct_depth (Overworld init)");

    for j in 0..h_u {
        for i in 0..w_u {
            let px = (x + i as i32) as f64;
            let pz = (z + j as i32) as f64;
            let mut off = oct_depth.sample_amp(px * 200.0, 10.0, pz * 200.0, 1.0, 0.0, true);
            off *= 65535.0 / 8000.0;
            if off < 0.0 {
                off = -0.3 * off;
            }
            off = off * 3.0 - 2.0;
            if off > 1.0 {
                off = 1.0;
            }
            off *= 17.0 / 64.0;
            if off < 0.0 {
                off *= 1.0 / 28.0;
            } else {
                off *= 1.0 / 40.0;
            }

            let mut vmin = 0.0_f64;
            let mut vmax = 0.0_f64;
            let mut ytest: i32 = 8;
            let mut ymin: i32 = 0;
            let mut ymax: i32 = 32;
            loop {
                let mut v = [0.0_f64; 2];
                for k in 0..2 {
                    let py = ytest + k as i32;
                    let mut n0 = sn.sample(x + i as i32, py, z + j as i32);
                    let mut fall = 1.0 - 2.0 * (py as f64) / 32.0 + off - 0.46875;
                    fall = scale_buf[j * w_u + i] * (fall + depth_buf[j * w_u + i]);
                    n0 += if fall > 0.0 { 4.0 * fall } else { fall };
                    v[k] = n0;
                    if n0 >= 0.0 && py > ymin {
                        ymin = py;
                        vmin = n0;
                    }
                    if n0 < 0.0 && py < ymax {
                        ymax = py;
                        vmax = n0;
                    }
                }
                let dy = v[0] / (v[0] - v[1]);
                // cubiomes: `dy = (dy <= 0 ? floor(dy) : ceil(dy))`
                // — round away from zero.
                let dy_rounded = if dy <= 0.0 { dy.floor() } else { dy.ceil() };
                ytest += dy_rounded as i32;
                if ytest <= ymin {
                    ytest = ymin + 1;
                }
                if ytest >= ymax {
                    ytest = ymax - 1;
                }
                if ymax - ymin <= 1 {
                    break;
                }
            }
            y[j * w_u + i] = (8.0 * (vmin / (vmin - vmax) + ymin as f64)) as f32;
        }
    }
}
