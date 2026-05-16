//! `getParaDescent` — gradient descent on a `DoublePerlinNoise`.
//!
//! Bit-exact port of cubiomes' `getParaDescent` from `finders.c`.
//! Walks a discrete (i, j) grid via 1-step probes plus opportunistic
//! larger jumps (`alpha * gradient`), returning the local minimum
//! of `factor * sample(x+i, 0, z+j)`. The optional callback is
//! invoked at every visited cell; returning `true` aborts the run
//! and the function returns `f64::NAN`.

#![allow(
    clippy::too_many_arguments,
    clippy::too_many_lines,
    clippy::cast_possible_truncation
)]

use crate::noise::double_perlin::DoublePerlinNoise;

/// Runs the gradient descent.
///
/// `factor` is multiplied into the sample value — pass `-1.0` to
/// search for a maximum instead. `alpha` is the jump-size scaler
/// (cubiomes uses 0–~10 depending on the climate axis).
///
/// # Example
///
/// ```
/// use cubioxides::biomenoise::BiomeNoise;
/// use cubioxides::biomenoise::biome_noise::NP_TEMPERATURE;
/// use cubioxides::finder::get_para_descent;
/// use cubioxides::MCVersion;
///
/// // Hill-climbing variant — `factor = 1.0` searches for a minimum;
/// // pass `-1.0` to flip the sign and find a maximum instead. The
/// // turbofish keeps type inference happy on the None-callback path.
/// let bn = BiomeNoise::new(MCVersion::V1_21, 0xdead_beef, false);
/// let temp = &bn.climate[NP_TEMPERATURE];
/// let _min_v = get_para_descent::<fn(i32, i32, f64) -> bool>(
///     temp, 1.0, 0, 0, 64, 64, 0, 0, 4, 32, 1.0, None,
/// );
/// ```
#[must_use]
pub fn get_para_descent<F>(
    para: &DoublePerlinNoise,
    factor: f64,
    x: i32,
    z: i32,
    w: i32,
    h: i32,
    i0: i32,
    j0: i32,
    maxrad: i32,
    maxiter: i32,
    alpha: f64,
    mut func: Option<F>,
) -> f64
where
    F: FnMut(i32, i32, f64) -> bool,
{
    let sample = |para: &DoublePerlinNoise, i: i32, j: i32| -> f64 {
        factor * para.sample(f64::from(x + i), 0.0, f64::from(z + j))
    };
    let mut v = sample(para, i0, j0);
    if let Some(f) = func.as_mut() {
        let reported = if factor < 0.0 { -v } else { v };
        if f(x + i0, z + j0, reported) {
            return f64::NAN;
        }
    }
    let mut i = i0;
    let mut j = j0;
    let mut dirx: i32 = 0;
    let mut dirz: i32 = 0;

    for _ in 0..maxiter {
        // x-axis probe
        if dirx == 0 {
            dirx = 1;
        }
        let mut vd = if i + dirx >= 0 && i + dirx < w {
            sample(para, i + dirx, j)
        } else {
            v
        };
        if vd >= v {
            dirx = -dirx;
            vd = if i + dirx >= 0 && i + dirx < w {
                sample(para, i + dirx, j)
            } else {
                v
            };
            if vd >= v {
                dirx = 0;
            }
        }
        if dirx != 0 {
            let dira = (f64::from(dirx) * alpha * (v - vd)) as i32;
            let mut moved = false;
            if dira.abs() > 2 && i + dira >= 0 && i + dira < w {
                let va = sample(para, i + dira, j);
                if va < vd {
                    i += dira;
                    v = va;
                    moved = true;
                }
            }
            if !moved {
                v = vd;
                i += dirx;
            }
            if let Some(f) = func.as_mut() {
                let reported = if factor < 0.0 { -v } else { v };
                if f(x + i, z + j, reported) {
                    return f64::NAN;
                }
            }
        }

        // z-axis probe
        if dirz == 0 {
            dirz = 1;
        }
        let mut vd = if j + dirz >= 0 && j + dirz < h {
            sample(para, i, j + dirz)
        } else {
            v
        };
        if vd >= v {
            dirz = -dirz;
            vd = if j + dirz >= 0 && j + dirz < h {
                sample(para, i, j + dirz)
            } else {
                v
            };
            if vd >= v {
                dirz = 0;
            }
        }
        if dirz != 0 {
            let dira = (f64::from(dirz) * alpha * (v - vd)) as i32;
            let mut moved = false;
            if dira.abs() > 2 && j + dira >= 0 && j + dira < h {
                let va = sample(para, i, j + dira);
                if va < vd {
                    j += dira;
                    v = va;
                    moved = true;
                }
            }
            if !moved {
                j += dirz;
                v = vd;
            }
            if let Some(f) = func.as_mut() {
                let reported = if factor < 0.0 { -v } else { v };
                if f(x + i, z + j, reported) {
                    return f64::NAN;
                }
            }
        }

        if dirx == 0 && dirz == 0 {
            // Diagonal probe — fixed-point check.
            let mut found = false;
            for c in 0..4 {
                let dx = if c & 1 != 0 { -1 } else { 1 };
                let dz = if c & 2 != 0 { -1 } else { 1 };
                if i + dx < 0 || i + dx >= w || j + dz < 0 || j + dz >= h {
                    continue;
                }
                let vd = sample(para, i + dx, j + dz);
                if vd < v {
                    v = vd;
                    i += dx;
                    j += dz;
                    dirx = dx;
                    dirz = dz;
                    found = true;
                    break;
                }
            }
            if !found {
                break;
            }
        }
        if (i - i0).abs() > maxrad || (j - j0).abs() > maxrad {
            break;
        }
    }
    v
}
