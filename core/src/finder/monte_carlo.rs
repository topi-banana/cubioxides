//! `monteCarloBiomes` — biome-coverage sampler with Wilson-score
//! early-exit. Bit-exact port of cubiomes' helper from `finders.c`.
//!
//! Given a [`Range`] over biome cells, samples random positions and
//! calls a user-provided `eval` closure to classify each sample.
//! Returns `true` when the success rate's lower Wilson bound exceeds
//! `coverage`, `false` when the upper bound falls below it. Aborts
//! and returns `false` immediately if `eval` returns an abort signal.
//!
//! When the sample budget approaches the total cell count, an
//! O(n) shuffle buffer is allocated so the same cell is never
//! visited twice. cubiomes' threshold is `n < 4 * wn && n < INT_MAX`;
//! we preserve that.

#![allow(clippy::missing_panics_doc, clippy::doc_markdown)]

use crate::generator::{Generator, Range};
use crate::math::{inverf, wilson};
use crate::rng::JavaRng;

/// Eval classification used by [`monte_carlo_biomes`].
#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MonteCarloEval {
    /// Skip this sample (no count toward `m`).
    Skip = -1,
    /// No-match.
    Fail = 0,
    /// Match.
    Success = 1,
    /// Abort the entire run; the function returns `false`.
    Abort = 2,
}

/// Bit-exact port of cubiomes' `monteCarloBiomes`.
///
/// The `eval` closure is called with `(generator, scale, x, y, z)`
/// for each sampled position. `r.sy == 0` is normalised to `1`
/// (matching cubiomes), and the function early-exits via the
/// Wilson score interval once the success rate's confidence
/// bounds clear `coverage` ± `1/m`.
#[allow(
    clippy::too_many_arguments,
    clippy::many_single_char_names,
    clippy::cast_precision_loss,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]
pub fn monte_carlo_biomes<F>(
    g: &Generator,
    range: Range,
    rng: &mut JavaRng,
    coverage: f64,
    confidence: f64,
    mut eval: F,
) -> bool
where
    F: FnMut(&Generator, i32, i32, i32, i32) -> MonteCarloEval,
{
    let sy = if range.sy == 0 { 1 } else { range.sy };
    let n = (range.sx as usize) * (sy as usize) * (range.sz as usize);
    if n == 0 {
        return true; // empty range — no work to do
    }

    let zscore = 2.0_f64.sqrt() * inverf(confidence);
    let wn = zscore * (n as f64).sqrt();
    let (wlo, whi) = wilson(wn, coverage, zscore);

    // Cubiomes allocates a tuple buffer when n is small relative to wn.
    let use_buf = (n as f64) < 4.0 * wn && i32::try_from(n).is_ok();
    let mut buf: Vec<(i32, i32, i32)> = if use_buf {
        let mut v = Vec::with_capacity(n);
        for k in 0..sy as i32 {
            for j in 0..range.sz as i32 {
                for i in 0..range.sx as i32 {
                    v.push((i, k, j));
                }
            }
        }
        v
    } else {
        Vec::new()
    };

    let mut m: f64 = 0.0;
    let mut x: f64 = 0.0;
    let mut ret = true;

    for i in 0..n {
        let (tx, ty, tz) = if use_buf {
            let remaining = (n - i) as i32;
            let k = rng.next_int(remaining) as usize;
            let last = n - i - 1;
            let pick = buf[k];
            // Swap k <-> last so future iterations pick from
            // [0..remaining-1] without revisiting `pick`.
            if k != last {
                buf[k] = buf[last];
                buf[last] = pick;
            }
            pick
        } else {
            let tx = rng.next_int(range.sx as i32);
            let ty = rng.next_int(sy as i32);
            let tz = rng.next_int(range.sz as i32);
            (tx, ty, tz)
        };

        let status = eval(g, range.scale, range.x + tx, range.y + ty, range.z + tz);
        match status {
            MonteCarloEval::Skip => continue,
            MonteCarloEval::Fail => {}
            MonteCarloEval::Success => x += 1.0,
            MonteCarloEval::Abort => {
                ret = false;
                break;
            }
        }
        m += 1.0;

        let per_m = 1.0 / m;
        let (lo, hi) = wilson(m, x * per_m, zscore);
        if lo - per_m > coverage {
            ret = true;
            break;
        }
        if hi + per_m < coverage {
            ret = false;
            break;
        }
        if hi - lo < whi - wlo {
            ret = x * per_m > coverage;
            break;
        }
    }
    ret
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mc_version::{Dimension, MCVersion};

    #[test]
    fn returns_true_for_empty_range() {
        let g = Generator::new(MCVersion::V1_18, 0);
        let r = Range {
            scale: 4,
            x: 0,
            z: 0,
            sx: 0,
            sz: 0,
            y: 0,
            sy: 1,
        };
        let mut rng = JavaRng::new(0);
        let ok = monte_carlo_biomes(&g, r, &mut rng, 0.5, 0.95, |_, _, _, _, _| {
            MonteCarloEval::Success
        });
        assert!(ok);
    }

    #[test]
    fn always_success_passes() {
        let mut g = Generator::new(MCVersion::V1_18, 0);
        g.apply_seed(Dimension::Overworld, 0xdead_beef);
        let r = Range {
            scale: 4,
            x: -64,
            z: -64,
            sx: 32,
            sz: 32,
            y: 0,
            sy: 1,
        };
        let mut rng = JavaRng::new(0xcafe);
        let ok = monte_carlo_biomes(&g, r, &mut rng, 0.5, 0.95, |_, _, _, _, _| {
            MonteCarloEval::Success
        });
        assert!(ok);
    }

    #[test]
    fn always_fail_returns_false() {
        let mut g = Generator::new(MCVersion::V1_18, 0);
        g.apply_seed(Dimension::Overworld, 0xdead_beef);
        let r = Range {
            scale: 4,
            x: 0,
            z: 0,
            sx: 16,
            sz: 16,
            y: 0,
            sy: 1,
        };
        let mut rng = JavaRng::new(42);
        let ok = monte_carlo_biomes(&g, r, &mut rng, 0.5, 0.95, |_, _, _, _, _| {
            MonteCarloEval::Fail
        });
        assert!(!ok);
    }
}
