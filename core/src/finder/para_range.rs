//! `getParaRange` — locate the min/max of a `DoublePerlinNoise`
//! over a 2D area by running `getParaDescent` from a grid of seeds.
//!
//! Bit-exact port of cubiomes' `getParaRange` from `finders.c`. The
//! algorithm uses the noise lacunarity to choose step sizes, then
//! prunes search starts via a skip-grid based on the maximum
//! noise-gradient contribution per step.

#![allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::cognitive_complexity,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::cell::Cell;

use crate::finder::para_descent::get_para_descent;
use crate::noise::double_perlin::DoublePerlinNoise;

/// Return value: the (`pmin`, `pmax`) min/max bound pair, or an
/// abort code if the caller's `func` requested early termination.
///
/// Cubiomes returns 0 on success and the callback's return value
/// on abort. We use `Result<(f64, f64), i32>` instead — `Ok` carries
/// the bounds, `Err(code)` propagates the callback's abort code.
pub type ParaRangeResult = Result<(f64, f64), i32>;

/// Bit-exact port of `getParaRange`.
///
/// `pmin_enabled` / `pmax_enabled` mirror cubiomes' "NULL pointer
/// disables this side" idiom. When a side is disabled, that bound
/// stays at its initial sentinel (`f64::MAX` / `-f64::MAX`).
///
/// # Example
///
/// ```
/// use cubioxides::biomenoise::BiomeNoise;
/// use cubioxides::biomenoise::biome_noise::NP_TEMPERATURE;
/// use cubioxides::finder::get_para_range;
/// use cubioxides::MCVersion;
///
/// // Scan a 16×16 cell window at the 1:4 grid for the min/max of
/// // the temperature axis. Pass `None` for the abort callback so the
/// // search runs to completion. Result is wrapped in Result —
/// // `Ok((pmin, pmax))` carries the bounds.
/// let bn = BiomeNoise::new(MCVersion::V1_21, 0xdead_beef, false);
/// let temp = &bn.climate[NP_TEMPERATURE];
/// let (_pmin, _pmax) = get_para_range::<fn(i32, i32, f64) -> i32>(
///     temp, true, true, 0, 0, 16, 16, None,
/// ).unwrap();
/// ```
pub fn get_para_range<F>(
    para: &DoublePerlinNoise,
    pmin_enabled: bool,
    pmax_enabled: bool,
    x: i32,
    z: i32,
    w: i32,
    h: i32,
    mut func: Option<F>,
) -> ParaRangeResult
where
    F: FnMut(i32, i32, f64) -> i32,
{
    const BETA: f64 = 1.5;
    const FACTOR: f64 = 10000.0;
    const PERLIN_GRAD: f64 = 2.0 * 1.875; // 3.75

    let mut pmin = f64::MAX;
    let mut pmax = -f64::MAX;

    // Lacunarity bounds across octA.
    let mut lmin = f64::MAX;
    let mut lmax: f64 = 0.0;
    for octave in &para.oct_a.octaves {
        if octave.lacunarity < lmin {
            lmin = octave.lacunarity;
        }
        if octave.lacunarity > lmax {
            lmax = octave.lacunarity;
        }
    }

    // Single wrapper for all callback invocations: stashes the abort
    // code into `abort_code` and reports `true` so descent / outer
    // loops can short-circuit.
    let abort_code: Cell<Option<i32>> = Cell::new(None);
    let mut wrap = |xi: i32, zj: i32, v: f64| -> bool {
        if let Some(f) = func.as_mut() {
            let code = f(xi, zj, v);
            if code != 0 {
                abort_code.set(Some(code));
                return true;
            }
        }
        false
    };

    let small_regime = 1e3 * lmax.sqrt();
    if (f64::from(w) * f64::from(h)) < small_regime {
        for j in 0..h {
            for i in 0..w {
                let v = FACTOR * para.sample(f64::from(x + i), 0.0, f64::from(z + j));
                if wrap(x + i, z + j, v) {
                    return Err(abort_code.get().unwrap_or(1));
                }
                if pmin_enabled && v < pmin {
                    pmin = v;
                }
                if pmax_enabled && v > pmax {
                    pmax = v;
                }
            }
        }
        return Ok((pmin, pmax));
    }

    let mut step = (0.5 / lmin - f64::from(f32::EPSILON)) as i32 + 1;
    let mut dr = lmax / lmin * BETA;

    // First pass: scan grid at the largest-period step, descend from each.
    let mut j = 0_i32;
    while j < h {
        let mut i = 0_i32;
        while i < w {
            if pmin_enabled {
                let v = get_para_descent(
                    para,
                    FACTOR,
                    x,
                    z,
                    w,
                    h,
                    i,
                    j,
                    step,
                    step,
                    dr,
                    Some(&mut wrap),
                );
                if v.is_nan() {
                    return Err(abort_code.get().unwrap_or(1));
                }
                if v < pmin {
                    pmin = v;
                }
            }
            if pmax_enabled {
                let v = -get_para_descent(
                    para,
                    -FACTOR,
                    x,
                    z,
                    w,
                    h,
                    i,
                    j,
                    step,
                    step,
                    dr,
                    Some(&mut wrap),
                );
                if v.is_nan() {
                    return Err(abort_code.get().unwrap_or(1));
                }
                if v > pmax {
                    pmax = v;
                }
            }
            i += step;
        }
        j += step;
    }

    // Second pass: finer step + skip-grid pruning.
    step = (1.0 / (PERLIN_GRAD * lmax + f64::from(f32::EPSILON))) as i32 + 1;

    let mut vdif = 0.0_f64;
    for octave in &para.oct_a.octaves {
        let mut contrib = f64::from(step) * octave.lacunarity;
        if contrib > 1.0 {
            contrib = 1.0;
        }
        vdif += contrib * octave.amplitude;
    }
    let lac_fact_b = 337.0 / 331.0;
    for octave in &para.oct_b.octaves {
        let mut contrib = f64::from(step) * octave.lacunarity * lac_fact_b;
        if contrib > 1.0 {
            contrib = 1.0;
        }
        vdif += contrib * octave.amplitude;
    }
    vdif = (FACTOR * vdif * para.amplitude).abs();

    let maxrad = step;
    let maxiter = step * 2;
    let ww = (w + step - 1) / step;
    let hh = (h + step - 1) / step;
    let skipsize = ((ww + 1) * (hh + 1)) as usize;

    if pmin_enabled {
        let mut skip = vec![false; skipsize];
        for jj in 0..=hh {
            let mut j = jj * step;
            if j >= h {
                j = h - 1;
            }
            for ii in 0..=ww {
                let mut i = ii * step;
                if i >= w {
                    i = w - 1;
                }
                if skip[(jj * ww + ii) as usize] {
                    continue;
                }
                let v = FACTOR * para.sample(f64::from(x + i), 0.0, f64::from(z + j));
                if wrap(x + i, z + j, v) {
                    return Err(abort_code.get().unwrap_or(1));
                }
                if pmax_enabled && v > pmax {
                    pmax = v;
                }
                dr = BETA * (v - pmin) / vdif;
                if dr > 1.0 {
                    let r = dr as i32;
                    for b in 0..r {
                        if b + jj < 0 || b + jj >= hh {
                            continue;
                        }
                        for a in (-r + 1)..r {
                            if a + ii < 0 || a + ii >= ww {
                                continue;
                            }
                            skip[((b + jj) * ww + (a + ii)) as usize] = true;
                        }
                    }
                    continue;
                }
                let v = get_para_descent(
                    para,
                    FACTOR,
                    x,
                    z,
                    w,
                    h,
                    i,
                    j,
                    maxrad,
                    maxiter,
                    dr,
                    Some(&mut wrap),
                );
                if v.is_nan() {
                    return Err(abort_code.get().unwrap_or(1));
                }
                if v < pmin {
                    pmin = v;
                }
            }
        }
    }

    if pmax_enabled {
        let mut skip = vec![false; skipsize];
        for jj in 0..=hh {
            let mut j = jj * step;
            if j >= h {
                j = h - 1;
            }
            for ii in 0..=ww {
                let mut i = ii * step;
                if i >= w {
                    i = w - 1;
                }
                if skip[(jj * ww + ii) as usize] {
                    continue;
                }
                let v = -FACTOR * para.sample(f64::from(x + i), 0.0, f64::from(z + j));
                if wrap(x + i, z + j, -v) {
                    return Err(abort_code.get().unwrap_or(1));
                }
                dr = BETA * (v + pmax) / vdif;
                if dr > 1.0 {
                    let r = dr as i32;
                    for b in 0..r {
                        if b + jj < 0 || b + jj >= hh {
                            continue;
                        }
                        for a in (-r + 1)..r {
                            if a + ii < 0 || a + ii >= ww {
                                continue;
                            }
                            skip[((b + jj) * ww + (a + ii)) as usize] = true;
                        }
                    }
                    continue;
                }
                let v = -get_para_descent(
                    para,
                    -FACTOR,
                    x,
                    z,
                    w,
                    h,
                    i,
                    j,
                    maxrad,
                    maxiter,
                    dr,
                    Some(&mut wrap),
                );
                if v.is_nan() {
                    return Err(abort_code.get().unwrap_or(1));
                }
                if v > pmax {
                    pmax = v;
                }
            }
        }
    }

    Ok((pmin, pmax))
}
