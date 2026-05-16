//! 1.9+ End biome noise (`EndNoise`).
//!
//! Bit-exact port of cubiomes' `setEndSeed`, `mapEndBiome`, `mapEnd`,
//! and the static `getEndBiome` height-map lookup. A single
//! Simplex-2D Perlin noise stack drives a precomputed 25×25 elevation
//! window; the closest non-zero elevation cell wins, and a 4-level
//! threshold maps it to one of the four End biomes
//! (`end_highlands`, `end_midlands`, `end_barrens`,
//! `small_end_islands`). The central 64-block disk is always
//! `the_end`.

#![allow(clippy::many_single_char_names)]

use crate::biome::Biome;
use crate::mc_version::MCVersion;
use crate::noise::PerlinNoise;
use crate::rng::JavaRng;

/// 25-entry square-distance lookup table from cubiomes:
/// `(25 - 2 * i) ^ 2` for `i = 0..=25`. The first entry handles the
/// `hx < 0` / `hz < 0` half-cell shift cubiomes applies via
/// `ds + (h < 0)`.
const DS: [u16; 26] = [
    625, 529, 441, 361, 289, 225, 169, 121, 81, 49, 25, 9, 1, 1, 9, 25, 49, 81, 121, 169, 225, 289,
    361, 441, 529, 625,
];

/// 1.9+ End biome noise. Built by [`EndNoise::set_seed`].
#[derive(Debug, Clone)]
pub struct EndNoise {
    /// Simplex-2D Perlin generator, sampled via
    /// `PerlinNoise::sample_simplex_2d`.
    pub perlin: PerlinNoise,
    /// MC version (affects the post-1.13 outer-ring fallback).
    pub mc: MCVersion,
}

impl EndNoise {
    /// `setEndSeed(en, mc, seed)` — seed a Java RNG with `seed`,
    /// skip 17,292 calls, then `perlinInit`.
    ///
    /// # Example
    ///
    /// ```
    /// use cubioxides::MCVersion;
    /// use cubioxides::biomenoise::EndNoise;
    ///
    /// // The End uses a single simplex-2D Perlin noise sampled at
    /// // a fixed RNG offset (17292 calls in). Once seeded, the same
    /// // `map_end_biome` output is reproducible across platforms.
    /// let en = EndNoise::set_seed(MCVersion::V1_21, 0xdead_beef);
    /// let mut out = vec![0i32; 16 * 16];
    /// en.map_end_biome(&mut out, 0, 0, 16, 16);
    /// ```
    #[must_use]
    pub fn set_seed(mc: MCVersion, seed: u64) -> Self {
        let mut rng = JavaRng::new(seed);
        rng.skip_n(17292);
        let perlin = PerlinNoise::from_java(&mut rng);
        Self { perlin, mc }
    }

    /// `mapEndBiome(en, out, x, z, w, h)` — fill `out` with End
    /// biome ids over a `(w, h)` grid at 1:16 scale (the native
    /// scale of `getEndBiome`'s height map).
    pub fn map_end_biome(&self, out: &mut [i32], x: i32, z: i32, w: usize, h: usize) {
        assert!(out.len() >= w * h, "map_end_biome: out slice too small");

        let hw = w + 26;
        let hh = h + 26;
        let mut hmap = vec![0u16; hw * hh];

        for j in 0..hh as i32 {
            for i in 0..hw as i32 {
                let rx = i64::from(x + i - 12);
                let rz = i64::from(z + j - 12);
                let rsq = (rx * rx + rz * rz) as u64;
                let mut v: u16 = 0;
                if rsq > 4096 {
                    let s = self.perlin.sample_simplex_2d(rx as f64, rz as f64);
                    if (s as f32) < -0.9_f32 {
                        // Mirror cubiomes' f32 arithmetic exactly.
                        let abs_rx = (rx as f32).abs();
                        let abs_rz = (rz as f32).abs();
                        let mag = (abs_rx * 3439.0 + abs_rz * 147.0) as u32;
                        let mut vv = mag % 13 + 9;
                        vv *= vv;
                        v = vv as u16;
                    }
                }
                hmap[j as usize * hw + i as usize] = v;
            }
        }

        for j in 0..h as i32 {
            for i in 0..w as i32 {
                let mut hx = i64::from(i + x);
                let mut hz = i64::from(j + z);
                let rsq = (hx * hx + hz * hz) as u64;

                if rsq <= 4096 {
                    out[j as usize * w + i as usize] = Biome::THE_END.id();
                    continue;
                }

                hx = 2 * hx + 1;
                hz = 2 * hz + 1;

                if self.mc.is_at_least(MCVersion::V1_14) {
                    // Cubiomes: `if (en->mc > MC_1_13)` — i.e. 1.14+.
                    // Outer-ring fallback when the squared distance
                    // overflows i32 (cubiomes casts `rsq` to int and
                    // checks `< 0`).
                    let rsq_i32 = (hx * hx + hz * hz) as i32;
                    if rsq_i32 < 0 {
                        out[j as usize * w + i as usize] = Biome::END_BARRENS.id();
                        continue;
                    }
                }

                // Index into hmap at (hz/2 - z, hx/2 - x), reading
                // the 25-cell square starting there.
                let row = (hz / 2 - i64::from(z)) as usize;
                let col = (hx / 2 - i64::from(x)) as usize;
                let id = get_end_biome(hx, hz, &hmap, hw, row, col);
                out[j as usize * w + i as usize] = id;
            }
        }
    }

    /// `mapEnd(en, out, x, z, w, h)` — 1:4 scale wrapper around
    /// [`Self::map_end_biome`].
    pub fn map_end(&self, out: &mut [i32], x: i32, z: i32, w: usize, h: usize) {
        assert!(out.len() >= w * h, "map_end: out slice too small");
        let cx = x >> 2;
        let cz = z >> 2;
        let cw = (((x + w as i32) >> 2) + 1 - cx) as usize;
        let ch = (((z + h as i32) >> 2) + 1 - cz) as usize;
        let mut buf = vec![0i32; cw * ch];
        self.map_end_biome(&mut buf, cx, cz, cw, ch);
        for j in 0..h as i32 {
            let cj = (((z + j) >> 2) - cz) as usize;
            for i in 0..w as i32 {
                let ci = (((x + i) >> 2) - cx) as usize;
                out[j as usize * w + i as usize] = buf[cj * cw + ci];
            }
        }
    }

    /// `getEndHeightNoise(en, x, z, range)` — sample the End surface
    /// height. `(x, z)` are in 8-block-per-cell coordinates (cubiomes'
    /// "noise space"). `range = 0` defaults to 12 cells around the
    /// sample point. Returns a height clamped to `[-100, 80]`.
    ///
    /// # Example
    ///
    /// ```
    /// use cubioxides::MCVersion;
    /// use cubioxides::biomenoise::EndNoise;
    ///
    /// // Raw noise-space height at the End origin. The 8-blocks-per-cell
    /// // coordinate convention is non-obvious — pass world coords >> 3,
    /// // not raw blocks. The returned f32 is always within [-100, 80].
    /// let en = EndNoise::set_seed(MCVersion::V1_21, 0xdead_beef);
    /// let h = en.end_height_noise(0, 0, 0);
    /// assert!((-100.0..=80.0).contains(&h));
    /// ```
    #[must_use]
    pub fn end_height_noise(&self, x: i32, z: i32, range: i32) -> f32 {
        let hx = x / 2;
        let hz = z / 2;
        let oddx = (x % 2) as i64;
        let oddz = (z % 2) as i64;

        let mut h: i64 = 64 * ((x as i64) * (x as i64) + (z as i64) * (z as i64));
        let range = if range == 0 { 12 } else { range };

        for j in -range..=range {
            for i in -range..=range {
                let rx = (hx + i) as i64;
                let rz = (hz + j) as i64;
                let rsq = (rx * rx + rz * rz) as u64;
                if rsq > 4096
                    && (self.perlin.sample_simplex_2d(rx as f64, rz as f64) as f32) < -0.9_f32
                {
                    let v = (((rx as f32).abs() * 3439.0_f32 + (rz as f32).abs() * 147.0_f32)
                        as u32
                        % 13
                        + 9) as i64;
                    let rx2 = oddx - (i as i64) * 2;
                    let rz2 = oddz - (j as i64) * 2;
                    let rsq2 = rx2 * rx2 + rz2 * rz2;
                    let noise = rsq2 * v * v;
                    if noise < h {
                        h = noise;
                    }
                }
            }
        }

        // cubiomes uses two separate `if` checks; `.clamp()` is bit-
        // exact equivalent for both finite and NaN inputs.
        (100.0_f32 - (h as f32).sqrt()).clamp(-100.0, 80.0)
    }
}

/// Closest-non-zero-elevation lookup over a 25×25 window starting
/// at `(row, col)` in `hmap`. Mirrors cubiomes' static `getEndBiome`.
fn get_end_biome(hx: i64, hz: i64, hmap: &[u16], hw: usize, row: usize, col: usize) -> i32 {
    let p_dsi_off = usize::from(hx < 0);
    let p_dsj_off = usize::from(hz < 0);

    let mut h: u32 = if hx.abs() <= 15 && hz.abs() <= 15 {
        (64 * (hx * hx + hz * hz)) as u32
    } else {
        14401
    };

    for j in 0..25usize {
        let dsj = u32::from(DS[p_dsj_off + j]);
        let row_base = (row + j) * hw + col;
        for i in 0..25usize {
            let e = u32::from(hmap[row_base + i]);
            if e != 0 {
                let u = (u32::from(DS[p_dsi_off + i]) + dsj) * e;
                if u < h {
                    h = u;
                }
            }
        }
    }

    if h < 3600 {
        Biome::END_HIGHLANDS.id()
    } else if h <= 10000 {
        Biome::END_MIDLANDS.id()
    } else if h <= 14400 {
        Biome::END_BARRENS.id()
    } else {
        Biome::SMALL_END_ISLANDS.id()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn central_disc_is_the_end() {
        let en = EndNoise::set_seed(MCVersion::V1_18, 1);
        let mut out = vec![0i32; 4];
        en.map_end_biome(&mut out, 0, 0, 2, 2);
        for v in &out {
            assert_eq!(*v, Biome::THE_END.id());
        }
    }

    #[test]
    fn deterministic_for_same_seed() {
        let a = EndNoise::set_seed(MCVersion::V1_18, 42);
        let b = EndNoise::set_seed(MCVersion::V1_18, 42);
        let mut out_a = vec![0i32; 64];
        let mut out_b = vec![0i32; 64];
        a.map_end_biome(&mut out_a, 100, 100, 8, 8);
        b.map_end_biome(&mut out_b, 100, 100, 8, 8);
        assert_eq!(out_a, out_b);
    }
}
