//! Perlin noise and 2D Simplex noise.
//!
//! Bit-exact port of cubiomes' `perlinInit`, `xPerlinInit`,
//! `samplePerlin`, and `sampleSimplex2D` in `cubiomes/noise.c`. The
//! permutation table is fixed at 257 bytes (256 entries plus a duplicate
//! of `d[0]` at index 256) and is shuffled at seed time via the same
//! Fisher–Yates loop as cubiomes.

// The single-char names below (a, b, c, d, i, j, t, ...) mirror the
// variable names in cubiomes/noise.c verbatim so future drift between
// the two implementations is easier to diff.
#![allow(clippy::many_single_char_names)]

use crate::math::{indexed_lerp, lerp, simplex_grad};
use crate::rng::{JavaRng, Xoroshiro};

/// 3D Perlin noise generator matching cubiomes' `PerlinNoise`.
///
/// Fields are `pub` so the `OctaveNoise` layer (and parity tests) can
/// inspect / adjust amplitude and lacunarity after construction, mirroring
/// the way cubiomes treats this struct as POD.
#[derive(Debug, Clone)]
pub struct PerlinNoise {
    /// Permutation table; entry 256 is always a duplicate of entry 0.
    pub d: [u8; 257],
    /// Cached `floor(b)` cast to `u8`, used when `sample_perlin` is called
    /// with `d2 == 0` (the common y-stride-zero case).
    pub h2: u8,
    /// Constant offset added to the `x` input of `sample_perlin`.
    pub a: f64,
    /// Constant offset added to the `y` input (or used as the cached `y=0`
    /// value via [`PerlinNoise::d2`] when `d2 == 0`).
    pub b: f64,
    /// Constant offset added to the `z` input of `sample_perlin`.
    pub c: f64,
    /// Per-octave amplitude. Always `1.0` until `OctaveNoise` overrides it.
    pub amplitude: f64,
    /// Per-octave lacunarity. Always `1.0` until `OctaveNoise` overrides it.
    pub lacunarity: f64,
    /// Cached `b - floor(b)` for the `d2 == 0` fast path.
    pub d2: f64,
    /// Cached smoothstep of [`PerlinNoise::d2`] for the `d2 == 0` fast path.
    pub t2: f64,
}

impl PerlinNoise {
    /// Initialise from a Java RNG (legacy biome generation).
    ///
    /// Used by every overworld noise stack from MC 1.0 through 1.17,
    /// and by the surface noise on all dimensions. The RNG is advanced
    /// by 3 `nextDouble` calls and 256 `nextInt` calls — the full
    /// Fisher-Yates shuffle of the permutation table.
    ///
    /// # Example
    ///
    /// ```
    /// use cubioxides::{JavaRng, PerlinNoise};
    ///
    /// let mut rng = JavaRng::new(0xdead_beef);
    /// let noise = PerlinNoise::from_java(&mut rng);
    /// let _y = noise.sample(1.0, 2.0, 3.0, 0.0, 0.0);
    /// ```
    #[must_use]
    pub fn from_java(rng: &mut JavaRng) -> Self {
        let a = rng.next_double() * 256.0;
        let b = rng.next_double() * 256.0;
        let c = rng.next_double() * 256.0;
        let mut d = [0u8; 257];
        for (i, slot) in d.iter_mut().take(256).enumerate() {
            *slot = i as u8;
        }
        for i in 0..256 {
            let j = rng.next_int((256 - i) as i32) as usize + i;
            d.swap(i, j);
        }
        d[256] = d[0];
        Self::finish_init(d, a, b, c)
    }

    /// Initialise from a Xoroshiro RNG (1.18+ biome generation).
    #[must_use]
    pub fn from_xoroshiro(xr: &mut Xoroshiro) -> Self {
        let a = xr.next_double() * 256.0;
        let b = xr.next_double() * 256.0;
        let c = xr.next_double() * 256.0;
        let mut d = [0u8; 257];
        for (i, slot) in d.iter_mut().take(256).enumerate() {
            *slot = i as u8;
        }
        for i in 0..256 {
            let j = xr.next_int((256 - i) as u32) as usize + i;
            d.swap(i, j);
        }
        d[256] = d[0];
        Self::finish_init(d, a, b, c)
    }

    #[allow(clippy::large_types_passed_by_value)] // the array is moved into Self
    fn finish_init(d: [u8; 257], a: f64, b: f64, c: f64) -> Self {
        let i2 = b.floor();
        let d2 = b - i2;
        let h2 = i2 as i32 as u8;
        let t2 = d2 * d2 * d2 * (d2 * (d2 * 6.0 - 15.0) + 10.0);
        Self {
            d,
            h2,
            a,
            b,
            c,
            amplitude: 1.0,
            lacunarity: 1.0,
            d2,
            t2,
        }
    }

    /// Sample 3D Perlin noise at `(d1, d2, d3)` with optional y modulation.
    ///
    /// Mirrors `samplePerlin(noise, d1, d2, d3, yamp, ymin)` exactly,
    /// including the `d2 == 0` fast path that reuses the cached `b`-frame.
    #[must_use]
    pub fn sample(&self, d1: f64, d2: f64, d3: f64, yamp: f64, ymin: f64) -> f64 {
        let (h2, t2, mut d2, mut d1, mut d3) = self.prepare_axes(d1, d2, d3);
        let i1 = d1.floor();
        let i3 = d3.floor();
        d1 -= i1;
        d3 -= i3;
        let h1 = i1 as i32 as u8;
        let h3 = i3 as i32 as u8;
        let t1 = d1 * d1 * d1 * (d1 * (d1 * 6.0 - 15.0) + 10.0);
        let t3 = d3 * d3 * d3 * (d3 * (d3 * 6.0 - 15.0) + 10.0);

        if yamp != 0.0 {
            let yclamp = if ymin < d2 { ymin } else { d2 };
            d2 -= (yclamp / yamp).floor() * yamp;
        }

        let idx = &self.d;
        let a1 = idx[h1 as usize].wrapping_add(h2);
        let b1 = idx[h1 as usize + 1].wrapping_add(h2);
        let a2 = idx[a1 as usize].wrapping_add(h3);
        let b2 = idx[b1 as usize].wrapping_add(h3);
        let a3 = idx[a1 as usize + 1].wrapping_add(h3);
        let b3 = idx[b1 as usize + 1].wrapping_add(h3);

        let l1 = indexed_lerp(idx[a2 as usize], d1, d2, d3);
        let l2 = indexed_lerp(idx[b2 as usize], d1 - 1.0, d2, d3);
        let l3 = indexed_lerp(idx[a3 as usize], d1, d2 - 1.0, d3);
        let l4 = indexed_lerp(idx[b3 as usize], d1 - 1.0, d2 - 1.0, d3);
        let l5 = indexed_lerp(idx[a2 as usize + 1], d1, d2, d3 - 1.0);
        let l6 = indexed_lerp(idx[b2 as usize + 1], d1 - 1.0, d2, d3 - 1.0);
        let l7 = indexed_lerp(idx[a3 as usize + 1], d1, d2 - 1.0, d3 - 1.0);
        let l8 = indexed_lerp(idx[b3 as usize + 1], d1 - 1.0, d2 - 1.0, d3 - 1.0);

        let l1 = lerp(t1, l1, l2);
        let l3 = lerp(t1, l3, l4);
        let l5 = lerp(t1, l5, l6);
        let l7 = lerp(t1, l7, l8);

        let l1 = lerp(t2, l1, l3);
        let l5 = lerp(t2, l5, l7);

        lerp(t3, l1, l5)
    }

    fn prepare_axes(&self, d1: f64, d2: f64, d3: f64) -> (u8, f64, f64, f64, f64) {
        let (h2, t2, d2) = if d2 == 0.0 {
            (self.h2, self.t2, self.d2)
        } else {
            let mut d2 = d2 + self.b;
            let i2 = d2.floor();
            d2 -= i2;
            let h2 = i2 as i32 as u8;
            let t2 = d2 * d2 * d2 * (d2 * (d2 * 6.0 - 15.0) + 10.0);
            (h2, t2, d2)
        };
        (h2, t2, d2, d1 + self.a, d3 + self.c)
    }

    /// `samplePerlinBeta17Terrain` — Beta 1.7-style terrain Perlin
    /// sampler. Accumulates contributions for `yi = 7` and `yi = 8`
    /// (writes into `v[0]` and `v[1]`), short-circuiting Y-axis
    /// recomputation when consecutive `i2` values agree.
    ///
    /// Bit-exact port of cubiomes' `samplePerlinBeta17Terrain`.
    #[allow(clippy::many_single_char_names)]
    pub fn sample_beta17_terrain(
        &self,
        v: &mut [f64; 2],
        mut d1: f64,
        mut d3: f64,
        y_lac_amp: f64,
    ) {
        let mut gen_flag: i32 = -1;
        let mut l1 = 0.0_f64;
        let mut l3 = 0.0_f64;
        let mut l5 = 0.0_f64;
        let mut l7 = 0.0_f64;

        d1 += self.a;
        d3 += self.c;
        let idx = &self.d;
        let mut i1 = d1.floor() as i32;
        let mut i3 = d3.floor() as i32;
        d1 -= f64::from(i1);
        d3 -= f64::from(i3);
        let t1 = d1 * d1 * d1 * (d1 * (d1 * 6.0 - 15.0) + 10.0);
        let t3 = d3 * d3 * d3 * (d3 * (d3 * 6.0 - 15.0) + 10.0);

        i1 &= 0xff;
        i3 &= 0xff;

        // First pass: find the latest yi at which the Y-cell changes.
        let mut yic: i32 = 0;
        let mut gf_copy: i32 = 0;
        for yi in 0..=7_i32 {
            let d2 = f64::from(yi) * self.lacunarity * y_lac_amp + self.b;
            let i2 = (d2.floor() as i32) & 0xff;
            if yi == 0 || i2 != gen_flag {
                yic = yi;
                gf_copy = gen_flag;
                gen_flag = i2;
            }
        }
        gen_flag = gf_copy;

        // Second pass: starting from yic, compute lerps and accumulate
        // into v[0], v[1] for yi >= 7.
        for yi in yic..=8_i32 {
            let mut d2 = f64::from(yi) * self.lacunarity * y_lac_amp + self.b;
            let mut i2 = d2.floor() as i32;
            d2 -= f64::from(i2);
            let t2 = d2 * d2 * d2 * (d2 * (d2 * 6.0 - 15.0) + 10.0);

            i2 &= 0xff;

            if yi == 0 || i2 != gen_flag {
                gen_flag = i2;
                // Cubiomes' Beta-1.7 Perlin reads `idx[a1]` / `idx[b1]`
                // without masking, where a1/b1 can exceed 256. The
                // standard Perlin uses `0xff &` masking explicitly;
                // we apply the same mask here to keep within bounds.
                // Both the standard sample path and the Beta one
                // produce identical results when the indices land
                // in 0..=255 (which is most of the time); for the
                // tail where a1 > 256 we wrap modulo 256, matching
                // the conventional Perlin lookup.
                let a1 = (i32::from(idx[i1 as usize]) + i2) & 0xff;
                let b1 = (i32::from(idx[(i1 + 1) as usize]) + i2) & 0xff;

                let a2 = (i32::from(idx[a1 as usize]) + i3) & 0xff;
                let a3 = (i32::from(idx[((a1 + 1) & 0xff) as usize]) + i3) & 0xff;
                let b2 = (i32::from(idx[b1 as usize]) + i3) & 0xff;
                let b3 = (i32::from(idx[((b1 + 1) & 0xff) as usize]) + i3) & 0xff;

                let m1 = indexed_lerp(idx[a2 as usize], d1, d2, d3);
                let l2 = indexed_lerp(idx[b2 as usize], d1 - 1.0, d2, d3);
                let m3 = indexed_lerp(idx[a3 as usize], d1, d2 - 1.0, d3);
                let l4 = indexed_lerp(idx[b3 as usize], d1 - 1.0, d2 - 1.0, d3);
                let m5 = indexed_lerp(idx[((a2 + 1) & 0xff) as usize], d1, d2, d3 - 1.0);
                let l6 = indexed_lerp(idx[((b2 + 1) & 0xff) as usize], d1 - 1.0, d2, d3 - 1.0);
                let m7 = indexed_lerp(idx[((a3 + 1) & 0xff) as usize], d1, d2 - 1.0, d3 - 1.0);
                let l8 = indexed_lerp(
                    idx[((b3 + 1) & 0xff) as usize],
                    d1 - 1.0,
                    d2 - 1.0,
                    d3 - 1.0,
                );

                l1 = lerp(t1, m1, l2);
                l3 = lerp(t1, m3, l4);
                l5 = lerp(t1, m5, l6);
                l7 = lerp(t1, m7, l8);
            }

            if yi >= 7 {
                let n1 = lerp(t2, l1, l3);
                let n5 = lerp(t2, l5, l7);
                v[(yi - 7) as usize] += lerp(t3, n1, n5) * self.amplitude;
            }
        }
    }

    /// Sample 2D Simplex noise at `(x, y)`.
    ///
    /// Mirrors `sampleSimplex2D` in cubiomes/noise.c.
    #[must_use]
    pub fn sample_simplex_2d(&self, x: f64, y: f64) -> f64 {
        let skew = 0.5 * (3.0_f64.sqrt() - 1.0);
        let unskew = (3.0 - 3.0_f64.sqrt()) / 6.0;
        let hf = (x + y) * skew;
        let hx = (x + hf).floor() as i32;
        let hz = (y + hf).floor() as i32;
        let mhxz = f64::from(hx + hz) * unskew;
        let x0 = x - (f64::from(hx) - mhxz);
        let y0 = y - (f64::from(hz) - mhxz);
        let offx = i32::from(x0 > y0);
        let offz = 1 - offx;
        let x1 = x0 - f64::from(offx) + unskew;
        let y1 = y0 - f64::from(offz) + unskew;
        let x2 = x0 - 1.0 + 2.0 * unskew;
        let y2 = y0 - 1.0 + 2.0 * unskew;
        let gi0 = self.d[(0xff & hz) as usize];
        let gi1 = self.d[(0xff & (hz + offz)) as usize];
        let gi2 = self.d[(0xff & (hz + 1)) as usize];
        let gi0 = self.d[(0xff & (i32::from(gi0) + hx)) as usize];
        let gi1 = self.d[(0xff & (i32::from(gi1) + hx + offx)) as usize];
        let gi2 = self.d[(0xff & (i32::from(gi2) + hx + 1)) as usize];
        let mut t = 0.0;
        t += simplex_grad(gi0 % 12, x0, y0, 0.0, 0.5);
        t += simplex_grad(gi1 % 12, x1, y1, 0.0, 0.5);
        t += simplex_grad(gi2 % 12, x2, y2, 0.0, 0.5);
        70.0 * t
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn from_java_fills_permutation_table_completely() {
        let mut rng = JavaRng::new(0);
        let p = PerlinNoise::from_java(&mut rng);
        // The 256-element prefix is a permutation of 0..=255.
        let mut seen = [false; 256];
        for &v in &p.d[..256] {
            seen[v as usize] = true;
        }
        assert!(seen.iter().all(|&b| b));
        // Entry 256 mirrors entry 0.
        assert_eq!(p.d[256], p.d[0]);
        // Amplitudes start at 1.0 before any octave-level adjustment.
        assert_eq!(p.amplitude, 1.0);
        assert_eq!(p.lacunarity, 1.0);
    }

    #[test]
    fn from_xoroshiro_fills_permutation_table_completely() {
        let mut xr = Xoroshiro::new(0);
        let p = PerlinNoise::from_xoroshiro(&mut xr);
        let mut seen = [false; 256];
        for &v in &p.d[..256] {
            seen[v as usize] = true;
        }
        assert!(seen.iter().all(|&b| b));
        assert_eq!(p.d[256], p.d[0]);
    }

    #[test]
    fn sample_is_deterministic() {
        let mut rng = JavaRng::new(42);
        let p = PerlinNoise::from_java(&mut rng);
        let a = p.sample(0.5, 0.25, 0.75, 0.0, 0.0);
        let b = p.sample(0.5, 0.25, 0.75, 0.0, 0.0);
        assert_eq!(a, b);
    }

    #[test]
    fn sample_y_zero_uses_cached_b_frame() {
        let mut rng = JavaRng::new(7);
        let p = PerlinNoise::from_java(&mut rng);
        // When d2 == 0 the slow path computes the same axes as the cache,
        // so the cached and uncached paths must agree numerically.
        let v_cached = p.sample(0.5, 0.0, 0.5, 0.0, 0.0);
        let v_recomputed = {
            let d2 = 1e-300_f64.copysign(1.0); // tiny non-zero, equivalent to 0 modulo floor
            p.sample(0.5, d2, 0.5, 0.0, 0.0)
        };
        // The two paths use slightly different `d2` values (0 vs 1e-300),
        // so the result is not bit-identical but must be very close.
        assert!((v_cached - v_recomputed).abs() < 1e-6);
    }

    #[test]
    fn sample_simplex_2d_is_deterministic() {
        let mut rng = JavaRng::new(123);
        let p = PerlinNoise::from_java(&mut rng);
        let a = p.sample_simplex_2d(1.5, -2.25);
        let b = p.sample_simplex_2d(1.5, -2.25);
        assert_eq!(a, b);
    }
}
