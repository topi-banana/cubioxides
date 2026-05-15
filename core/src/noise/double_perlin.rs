//! Double Perlin noise (1.18+).
//!
//! Wraps two [`OctaveNoise`] stacks sampled at frequencies `1.0` and
//! `337/331`, weighted by an amplitude derived from the active octave
//! count. Bit-exact port of cubiomes' `doublePerlinInit`,
//! `xDoublePerlinInit`, and `sampleDoublePerlin`.

use crate::noise::OctaveNoise;
use crate::rng::{JavaRng, Xoroshiro};

/// Frequency offset applied to the second octave stack inside
/// `sample_double_perlin`. Matches the literal `337.0 / 331.0` in
/// cubiomes/noise.c.
const SECOND_STACK_FREQ: f64 = 337.0 / 331.0;

/// `(5.0 / 3.0) * len / (len + 1)` for `len = 0..=9`. Matches `amp_ini`
/// inside `xDoublePerlinInit`.
const AMP_INI: [f64; 10] = [
    0.0,
    5.0 / 6.0,
    10.0 / 9.0,
    15.0 / 12.0,
    20.0 / 15.0,
    25.0 / 18.0,
    30.0 / 21.0,
    35.0 / 24.0,
    40.0 / 27.0,
    45.0 / 30.0,
];

/// Twin-stack Perlin generator used by Minecraft 1.18+ biome noise.
#[derive(Debug, Clone)]
pub struct DoublePerlinNoise {
    /// Final per-sample amplitude (applied after summing the two stacks).
    pub amplitude: f64,
    /// First octave stack, sampled at the requested frequency.
    pub oct_a: OctaveNoise,
    /// Second octave stack, sampled at frequency `337/331` of the first.
    pub oct_b: OctaveNoise,
}

impl DoublePerlinNoise {
    /// Initialise from a Java RNG (`doublePerlinInit`).
    ///
    /// `len` must be at least 1 and `omin + len <= 0`.
    #[must_use]
    pub fn from_java(rng: &mut JavaRng, omin: i32, len: i32) -> Self {
        assert!(len >= 1, "double perlin len must be at least 1");
        let amplitude = (10.0 / 6.0) * f64::from(len) / f64::from(len + 1);
        let oct_a = OctaveNoise::from_java(rng, omin, len);
        let oct_b = OctaveNoise::from_java(rng, omin, len);
        Self {
            amplitude,
            oct_a,
            oct_b,
        }
    }

    /// Initialise from a Xoroshiro RNG (`xDoublePerlinInit`).
    ///
    /// `nmax` (when `Some`) caps the total number of non-zero octaves
    /// across both stacks; the cap is split as `(nmax + 1) / 2` for the
    /// first stack and the remainder for the second.
    #[must_use]
    pub fn from_xoroshiro(
        xr: &mut Xoroshiro,
        amplitudes: &[f64],
        omin: i32,
        nmax: Option<usize>,
    ) -> Self {
        let (nmax_a, nmax_b) = match nmax {
            Some(m) => {
                let na = m.div_ceil(2);
                let nb = m - na;
                (Some(na), Some(nb))
            }
            None => (None, None),
        };
        let oct_a = OctaveNoise::from_xoroshiro(xr, amplitudes, omin, nmax_a);
        let oct_b = OctaveNoise::from_xoroshiro(xr, amplitudes, omin, nmax_b);

        // Trim trailing and leading zero amplitudes so AMP_INI lookup uses
        // the count of active octaves.
        let trailing = amplitudes.iter().rev().take_while(|&&a| a == 0.0).count();
        let leading = amplitudes.iter().take_while(|&&a| a == 0.0).count();
        let trimmed = amplitudes
            .len()
            .saturating_sub(trailing)
            .saturating_sub(leading);
        let amplitude = AMP_INI[trimmed];

        Self {
            amplitude,
            oct_a,
            oct_b,
        }
    }

    /// Sample at `(x, y, z)`.
    ///
    /// Mirrors `sampleDoublePerlin`: sums the two stacks (the second
    /// stretched by `337/331`) and scales by `self.amplitude`.
    #[must_use]
    pub fn sample(&self, x: f64, y: f64, z: f64) -> f64 {
        let v = self.oct_a.sample(x, y, z)
            + self.oct_b.sample(
                x * SECOND_STACK_FREQ,
                y * SECOND_STACK_FREQ,
                z * SECOND_STACK_FREQ,
            );
        v * self.amplitude
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn from_java_uses_amplitude_formula() {
        let mut rng = JavaRng::new(0);
        let dp = DoublePerlinNoise::from_java(&mut rng, -3, 4);
        // (10/6) * 4 / 5 = 8 / 6 = 4/3
        assert!((dp.amplitude - (10.0 / 6.0) * 4.0 / 5.0).abs() < 1e-12);
        assert_eq!(dp.oct_a.len(), 4);
        assert_eq!(dp.oct_b.len(), 4);
    }

    #[test]
    fn from_xoroshiro_amp_uses_trimmed_length() {
        let mut xr = Xoroshiro::new(0);
        // Amplitudes [0, 1, 1, 0]: trimmed length is 2, AMP_INI[2] = 10/9.
        let amps = [0.0, 1.0, 1.0, 0.0];
        let dp = DoublePerlinNoise::from_xoroshiro(&mut xr, &amps, -3, None);
        assert!((dp.amplitude - 10.0 / 9.0).abs() < 1e-12);
    }

    #[test]
    fn from_xoroshiro_splits_nmax_across_stacks() {
        let mut xr = Xoroshiro::new(0);
        let amps = [1.0, 1.0, 1.0, 1.0];
        let dp = DoublePerlinNoise::from_xoroshiro(&mut xr, &amps, -3, Some(5));
        // ceil(5 / 2) = 3 for the first stack, 2 for the second.
        assert_eq!(dp.oct_a.len(), 3);
        assert_eq!(dp.oct_b.len(), 2);
    }

    #[test]
    fn sample_is_deterministic() {
        let mut rng = JavaRng::new(42);
        let dp = DoublePerlinNoise::from_java(&mut rng, -3, 4);
        let a = dp.sample(0.5, 0.25, 0.75);
        let b = dp.sample(0.5, 0.25, 0.75);
        assert_eq!(a, b);
    }
}
