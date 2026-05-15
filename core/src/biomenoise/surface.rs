//! 1.13+ `SurfaceNoise` — the 60-octave Perlin stack underpinning
//! Overworld + End surface height and density sampling.
//!
//! Bit-exact port of cubiomes' `initSurfaceNoise` +
//! `sampleSurfaceNoise` + `sampleSurfaceNoiseBetween`. Splits
//! cubiomes' single C struct into a Rust-friendly form: the four /
//! five `OctaveNoise` fields each own their `Vec<PerlinNoise>`, and
//! the Overworld-only `oct_surf` / `oct_depth` are `Option` because
//! cubiomes leaves them uninitialised for the End.

use crate::mc_version::Dimension;
use crate::noise::OctaveNoise;
use crate::rng::JavaRng;

/// Identity stand-in for cubiomes' `maintainPrecision`. Upstream
/// comments note the function is a no-op when sampling with `double`
/// inputs (the `round(x / 2^25) * 2^25` correction was for an older
/// `float` pipeline). Re-exported here so call sites mirror cubiomes
/// 1:1.
#[inline]
#[must_use]
pub fn maintain_precision(x: f64) -> f64 {
    x
}

/// 1.13+ surface noise. Built by [`SurfaceNoise::init`]; sampled by
/// [`SurfaceNoise::sample`] (the standard density column) and
/// [`SurfaceNoise::sample_between`] (the early-exit variant used for
/// upper / lower envelope clamps).
#[derive(Debug, Clone)]
pub struct SurfaceNoise {
    /// Horizontal density scale; 2.0 in the End, ≈1 elsewhere.
    pub xz_scale: f64,
    /// Vertical density scale; 1.0 in the End, ≈1 elsewhere.
    pub y_scale: f64,
    /// Horizontal step divisor (constant 80).
    pub xz_factor: f64,
    /// Vertical step divisor (constant 160).
    pub y_factor: f64,
    /// 16-octave lower envelope.
    pub oct_min: OctaveNoise,
    /// 16-octave upper envelope.
    pub oct_max: OctaveNoise,
    /// 8-octave main blend channel.
    pub oct_main: OctaveNoise,
    /// 4-octave surface scrambler (Overworld only).
    pub oct_surf: Option<OctaveNoise>,
    /// 16-octave depth modulator (Overworld only).
    pub oct_depth: Option<OctaveNoise>,
}

impl SurfaceNoise {
    /// Cubiomes' `initSurfaceNoise(sn, dim, seed)` — pull octaves from
    /// a Java RNG keyed by `seed`. In the End the four extra surface
    /// + depth octaves are skipped (and the scale constants change).
    #[must_use]
    pub fn init(dim: Dimension, seed: u64) -> Self {
        let mut rng = JavaRng::new(seed);
        let oct_min = OctaveNoise::from_java(&mut rng, -15, 16);
        let oct_max = OctaveNoise::from_java(&mut rng, -15, 16);
        let oct_main = OctaveNoise::from_java(&mut rng, -7, 8);

        if matches!(dim, Dimension::End) {
            Self {
                xz_scale: 2.0,
                y_scale: 1.0,
                xz_factor: 80.0,
                y_factor: 160.0,
                oct_min,
                oct_max,
                oct_main,
                oct_surf: None,
                oct_depth: None,
            }
        } else {
            let oct_surf = OctaveNoise::from_java(&mut rng, -3, 4);
            rng.skip_n(262 * 10);
            let oct_depth = OctaveNoise::from_java(&mut rng, -15, 16);
            Self {
                // Cubiomes encodes 0.9999999814507745 verbatim; Java's
                // double-to-string reproduces this exact bit pattern.
                xz_scale: 0.999_999_981_450_774_5,
                y_scale: 0.999_999_981_450_774_5,
                xz_factor: 80.0,
                y_factor: 160.0,
                oct_min,
                oct_max,
                oct_main,
                oct_surf: Some(oct_surf),
                oct_depth: Some(oct_depth),
            }
        }
    }

    /// Cubiomes' `sampleSurfaceNoise(sn, x, y, z)`.
    #[must_use]
    pub fn sample(&self, x: i32, y: i32, z: i32) -> f64 {
        let xz_scale = 684.412 * self.xz_scale;
        let y_scale = 684.412 * self.y_scale;
        let xz_step = xz_scale / self.xz_factor;
        let y_step = y_scale / self.y_factor;

        let mut min_noise = 0.0;
        let mut max_noise = 0.0;
        let mut main_noise = 0.0;
        let mut persist = 1.0;
        let mut contrib = 1.0;

        let x = f64::from(x);
        let y = f64::from(y);
        let z = f64::from(z);

        for i in 0..16 {
            let dx = maintain_precision(x * xz_scale * persist);
            let dy = maintain_precision(y * y_scale * persist);
            let dz = maintain_precision(z * xz_scale * persist);
            let sy = y_scale * persist;
            let ty = y * sy;

            min_noise += self.oct_min.octaves[i].sample(dx, dy, dz, sy, ty) * contrib;
            max_noise += self.oct_max.octaves[i].sample(dx, dy, dz, sy, ty) * contrib;

            if i < 8 {
                let dx = maintain_precision(x * xz_step * persist);
                let dy = maintain_precision(y * y_step * persist);
                let dz = maintain_precision(z * xz_step * persist);
                let sy = y_step * persist;
                let ty = y * sy;
                main_noise += self.oct_main.octaves[i].sample(dx, dy, dz, sy, ty) * contrib;
            }
            persist *= 0.5;
            contrib *= 2.0;
        }

        crate::math::clamped_lerp(
            0.5 + 0.05 * main_noise,
            min_noise / 512.0,
            max_noise / 512.0,
        )
    }

    /// Cubiomes' `sampleSurfaceNoiseBetween` — early-exits when the
    /// running min/max envelopes prove the column lies outside
    /// `[noise_min, noise_max]`.
    #[must_use]
    pub fn sample_between(&self, x: i32, y: i32, z: i32, noise_min: f64, noise_max: f64) -> f64 {
        let xz_scale = 684.412 * self.xz_scale;
        let y_scale = 684.412 * self.y_scale;

        let mut vmin = 0.0_f64;
        let mut vmax = 0.0_f64;

        let mut persist = 1.0 / 32768.0;
        let mut amp = 64.0;

        let xf = f64::from(x);
        let yf = f64::from(y);
        let zf = f64::from(z);

        for i in (0..16).rev() {
            let dx = xf * xz_scale * persist;
            let dz = zf * xz_scale * persist;
            let sy = y_scale * persist;
            let dy = yf * sy;

            vmin += self.oct_min.octaves[i].sample(dx, dy, dz, sy, dy) * amp;
            vmax += self.oct_max.octaves[i].sample(dx, dy, dz, sy, dy) * amp;
            if vmin - amp > noise_max && vmax - amp > noise_max {
                return noise_max;
            }
            if vmin + amp < noise_min && vmax + amp < noise_min {
                return noise_min;
            }

            amp *= 0.5;
            persist *= 2.0;
        }

        let xz_step = xz_scale / self.xz_factor;
        let y_step = y_scale / self.y_factor;
        let mut vmain = 0.5_f64;

        persist = 1.0 / 128.0;
        amp = 0.05 * 128.0;

        for i in (0..8).rev() {
            let dx = xf * xz_step * persist;
            let dz = zf * xz_step * persist;
            let sy = y_step * persist;
            let dy = yf * sy;

            vmain += self.oct_main.octaves[i].sample(dx, dy, dz, sy, dy) * amp;
            if vmain - amp > 1.0 {
                return vmax;
            }
            if vmain + amp < 0.0 {
                return vmin;
            }

            amp *= 0.5;
            persist *= 2.0;
        }

        crate::math::clamped_lerp(vmain, vmin, vmax)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_overworld_populates_surf_and_depth() {
        let sn = SurfaceNoise::init(Dimension::Overworld, 12345);
        assert!(sn.oct_surf.is_some());
        assert!(sn.oct_depth.is_some());
        assert_eq!(sn.oct_min.len(), 16);
        assert_eq!(sn.oct_max.len(), 16);
        assert_eq!(sn.oct_main.len(), 8);
        assert_eq!(sn.oct_surf.as_ref().unwrap().len(), 4);
        assert_eq!(sn.oct_depth.as_ref().unwrap().len(), 16);
    }

    #[test]
    fn init_end_skips_surf_and_depth() {
        let sn = SurfaceNoise::init(Dimension::End, 42);
        assert!(sn.oct_surf.is_none());
        assert!(sn.oct_depth.is_none());
        assert!((sn.xz_scale - 2.0).abs() < f64::EPSILON);
        assert!((sn.y_scale - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn sample_is_deterministic() {
        let sn = SurfaceNoise::init(Dimension::Overworld, 99);
        let a = sn.sample(10, 64, -7);
        let b = sn.sample(10, 64, -7);
        assert_eq!(a.to_bits(), b.to_bits());
    }
}
