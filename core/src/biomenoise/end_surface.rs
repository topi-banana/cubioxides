//! End-dimension surface height generation.
//!
//! Bit-exact port of `sampleNoiseColumnEnd`, `getSurfaceHeight`,
//! `mapEndSurfaceHeight`, and `getEndSurfaceHeight` from cubiomes'
//! `biomenoise.c`. Used by [`crate::finder::end::is_end_chunk_empty`]
//! and by the End spawn / end-city placement checks.

#![allow(
    clippy::many_single_char_names,
    clippy::cast_possible_wrap,
    clippy::too_many_arguments,
    clippy::invalid_upcast_comparisons
)]

use crate::biomenoise::end::EndNoise;
use crate::biomenoise::surface::SurfaceNoise;
use crate::math::{floordiv, lerp, lerp3};
use crate::mc_version::MCVersion;

/// `clamped (32 + 46 - y) / 64.0`. Index range: 0..=32.
const UPPER_DROP: [f64; 33] = [
    1.0,
    1.0,
    1.0,
    1.0,
    1.0,
    1.0,
    1.0,
    1.0,
    1.0,
    1.0,
    1.0,
    1.0,
    1.0,
    1.0,
    1.0,
    63.0 / 64.0,
    62.0 / 64.0,
    61.0 / 64.0,
    60.0 / 64.0,
    59.0 / 64.0,
    58.0 / 64.0,
    57.0 / 64.0,
    56.0 / 64.0,
    55.0 / 64.0,
    54.0 / 64.0,
    53.0 / 64.0,
    52.0 / 64.0,
    51.0 / 64.0,
    50.0 / 64.0,
    49.0 / 64.0,
    48.0 / 64.0,
    47.0 / 64.0,
    46.0 / 64.0,
];

/// `clamped (y - 1) / 7.0`. Index range: 0..=32.
const LOWER_DROP: [f64; 33] = [
    0.0,
    0.0,
    1.0 / 7.0,
    2.0 / 7.0,
    3.0 / 7.0,
    4.0 / 7.0,
    5.0 / 7.0,
    6.0 / 7.0,
    1.0,
    1.0,
    1.0,
    1.0,
    1.0,
    1.0,
    1.0,
    1.0,
    1.0,
    1.0,
    1.0,
    1.0,
    1.0,
    1.0,
    1.0,
    1.0,
    1.0,
    1.0,
    1.0,
    1.0,
    1.0,
    1.0,
    1.0,
    1.0,
    1.0,
];

/// `sampleNoiseColumnEnd` — fill `column` with `colymax - colymin + 1`
/// double-precision noise values at `(x, z)` in cell coords.
///
/// For MC > 1.13, an outer-ring fallback fills the column with NaN
/// when the squared distance overflows i32 (matching cubiomes' cast
/// to `int`).
pub fn sample_noise_column_end(
    column: &mut [f64],
    sn: &SurfaceNoise,
    en: &EndNoise,
    x: i32,
    z: i32,
    colymin: i32,
    colymax: i32,
) {
    let n = (colymax - colymin + 1) as usize;
    debug_assert!(column.len() >= n);

    if en.mc.is_at_least(MCVersion::V1_14) {
        // Add outer end rings: NaN-out columns beyond the i32 overflow.
        // cubiomes: `uint64_t rsq = (uint64_t) x * x + (uint64_t) z * z;`
        // where `x` is `int`. C sign-extends through (uint64_t), which
        // Rust spells as `as i64 as u64`.
        let rsq = (x as i64 as u64).wrapping_mul(x as i64 as u64)
            + (z as i64 as u64).wrapping_mul(z as i64 as u64);
        if (rsq as i32) < 0 {
            for slot in column.iter_mut().take(n) {
                *slot = f64::NAN;
            }
            return;
        }
    }

    let depth = en.end_height_noise(x, z, 0) as f64 - 8.0_f32 as f64;
    for y in colymin..=colymax {
        let idx = (y - colymin) as usize;
        if LOWER_DROP[y as usize] == 0.0 {
            column[idx] = -30.0;
            continue;
        }
        let noise = sn.sample_between(x, y, z, -128.0, 128.0);
        let mut clamped = noise + depth;
        clamped = lerp(UPPER_DROP[y as usize], -3000.0, clamped);
        clamped = lerp(LOWER_DROP[y as usize], -30.0, clamped);
        column[idx] = clamped;
    }
}

/// `getSurfaceHeight` — given four bordering noise columns and a
/// fractional `(dx, dz)` position between them, scan downward
/// through the cells to find the topmost block where the trilinear
/// interpolated noise is `> 0`. Returns 0 if no positive cell exists.
#[allow(clippy::too_many_arguments)]
pub fn get_surface_height(
    ncol00: &[f64],
    ncol01: &[f64],
    ncol10: &[f64],
    ncol11: &[f64],
    colymin: i32,
    colymax: i32,
    blockspercell: i32,
    dx: f64,
    dz: f64,
) -> i32 {
    for celly in (colymin..colymax).rev() {
        let idx = (celly - colymin) as usize;
        let v000 = ncol00[idx];
        let v001 = ncol01[idx];
        let v100 = ncol10[idx];
        let v101 = ncol11[idx];
        let v010 = ncol00[idx + 1];
        let v011 = ncol01[idx + 1];
        let v110 = ncol10[idx + 1];
        let v111 = ncol11[idx + 1];

        for y in (0..blockspercell).rev() {
            let dy = y as f64 / blockspercell as f64;
            let noise = lerp3(
                dy, dx, dz, // Note: cubiomes uses (dy, dx, dz) — not (dx, dy, dz).
                v000, v010, v100, v110, v001, v011, v101, v111,
            );
            if noise > 0.0 {
                return celly * blockspercell + y;
            }
        }
    }
    0
}

/// `mapEndSurfaceHeight(y, en, sn, x, z, w, h, scale, ymin)` — fill
/// `y` with per-pixel End surface heights over the `(w, h)` region.
/// `scale` must be 1, 2, 4, or 8. `ymin` clips the column-scan
/// bottom (cubiomes' `y0` is `clamp(ymin >> 2, 2, 17)`).
///
/// Returns 0 on success, 1 if `scale` is unsupported.
#[allow(clippy::too_many_arguments)]
pub fn map_end_surface_height(
    y: &mut [f32],
    en: &EndNoise,
    sn: &SurfaceNoise,
    x: i32,
    z: i32,
    w: i32,
    h: i32,
    scale: i32,
    ymin: i32,
) -> i32 {
    if scale != 1 && scale != 2 && scale != 4 && scale != 8 {
        return 1;
    }
    assert!(
        y.len() >= (w * h) as usize,
        "map_end_surface_height: y buffer too small ({}x{}={} vs len {})",
        w,
        h,
        w * h,
        y.len()
    );

    let y0 = (ymin >> 2).clamp(2, 17);
    let y1 = 18;
    let yn = (y1 - y0 + 1) as usize;
    let cellmid = if scale > 1 { scale as f64 / 16.0 } else { 0.0 };
    let cellsiz = 8 / scale;
    let cx = floordiv(x, cellsiz);
    let cz = floordiv(z, cellsiz);
    let cw = (floordiv(x + w - 1, cellsiz) - cx + 2) as usize;

    // Two row-buffers of `yn * cw` doubles each.
    let mut buf0 = vec![0.0_f64; yn * cw];
    let mut buf1 = vec![0.0_f64; yn * cw];

    // Prime the back-buffer with row cz.
    for i in 0..cw {
        let slot = &mut buf1[i * yn..(i + 1) * yn];
        sample_noise_column_end(slot, sn, en, cx + i as i32, cz, y0, y1);
    }

    for j in 0..h {
        let cj = floordiv(z + j, cellsiz);
        let dj = z + j - cj * cellsiz;
        if j == 0 || dj == 0 {
            // Advance the rolling window: ncol[0] := ncol[1], then
            // re-sample row (cj + 1) into ncol[1].
            std::mem::swap(&mut buf0, &mut buf1);
            for i in 0..cw {
                let slot = &mut buf1[i * yn..(i + 1) * yn];
                sample_noise_column_end(slot, sn, en, cx + i as i32, cj + 1, y0, y1);
            }
        }
        for i in 0..w {
            let ci = floordiv(x + i, cellsiz);
            let di = x + i - ci * cellsiz;
            let dx = di as f64 / cellsiz as f64 + cellmid;
            let dz = dj as f64 / cellsiz as f64 + cellmid;
            let ci_idx = (ci - cx) as usize;
            let ncol0 = &buf0[ci_idx * yn..];
            let ncol1 = &buf1[ci_idx * yn..];
            let ncol0_a = &ncol0[..yn];
            let ncol1_a = &ncol1[..yn];
            let ncol0_b = &buf0[(ci_idx + 1) * yn..(ci_idx + 2) * yn];
            let ncol1_b = &buf1[(ci_idx + 1) * yn..(ci_idx + 2) * yn];
            y[(j * w + i) as usize] =
                get_surface_height(ncol0_a, ncol1_a, ncol0_b, ncol1_b, y0, y1, 4, dx, dz) as f32;
        }
    }
    0
}

const Y0: i32 = 0;
const Y1: i32 = 32;
const YN: usize = (Y1 - Y0 + 1) as usize;

/// `getEndSurfaceHeight(mc, seed, x, z)` — convenience wrapper.
/// Constructs `EndNoise` + `SurfaceNoise` from `(mc, seed)` and
/// returns the surface block height at `(x, z)`.
#[must_use]
pub fn get_end_surface_height(mc: MCVersion, seed: u64, x: i32, z: i32) -> i32 {
    let en = EndNoise::set_seed(mc, seed);
    let sn = SurfaceNoise::init(crate::mc_version::Dimension::End, seed);

    let cellx = x >> 3;
    let cellz = z >> 3;
    let dx = (x & 7) as f64 / 8.0;
    let dz = (z & 7) as f64 / 8.0;

    let mut ncol00 = [0.0_f64; YN];
    let mut ncol01 = [0.0_f64; YN];
    let mut ncol10 = [0.0_f64; YN];
    let mut ncol11 = [0.0_f64; YN];
    sample_noise_column_end(&mut ncol00, &sn, &en, cellx, cellz, Y0, Y1);
    sample_noise_column_end(&mut ncol01, &sn, &en, cellx, cellz + 1, Y0, Y1);
    sample_noise_column_end(&mut ncol10, &sn, &en, cellx + 1, cellz, Y0, Y1);
    sample_noise_column_end(&mut ncol11, &sn, &en, cellx + 1, cellz + 1, Y0, Y1);

    get_surface_height(&ncol00, &ncol01, &ncol10, &ncol11, Y0, Y1, 4, dx, dz)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn end_surface_height_finite() {
        let h = get_end_surface_height(MCVersion::V1_18, 0xdead_beef, 100, 100);
        assert!((0..256).contains(&h), "height out of range: {h}");
    }
}
