//! Octave-stacked Perlin noise.
//!
//! Bit-exact port of cubiomes' `OctaveNoise` family: `octaveInit`,
//! `octaveInitBeta`, `xOctaveInit`, and `sampleOctave*`. Each octave is a
//! [`PerlinNoise`] with its own amplitude and lacunarity; `sample` returns
//! the amplitude-weighted sum across all octaves.

use crate::noise::PerlinNoise;
use crate::rng::{JavaRng, Xoroshiro};

/// Stafford-mix XOR keys derived from the MD5 of the string `octave_N` for
/// `N = -12..=0`. Matches `md5_octave_n` in cubiomes/noise.c.
pub const MD5_OCTAVE_N: [[u64; 2]; 13] = [
    [0xb198_de63_a801_2672, 0x7b84_cad4_3ef7_b5a8], // octave_-12
    [0x0fd7_87bf_bc40_3ec3, 0x74a4_a31c_a21b_48b8], // octave_-11
    [0x36d3_26ee_d40e_feb2, 0x5be9_ce18_223c_636a], // octave_-10
    [0x082f_e255_f8be_6631, 0x4e96_119e_22de_dc81], // octave_-9
    [0x0ef6_8ec6_8504_005e, 0x48b6_bf93_a278_9640], // octave_-8
    [0xf112_6812_8982_754f, 0x257a_1d67_0430_b0aa], // octave_-7
    [0xe51c_98ce_7d1d_e664, 0x5f94_78a7_3304_0c45], // octave_-6
    [0x6d7b_49e7_e429_850a, 0x2e30_63c6_22a2_4777], // octave_-5
    [0xbd90_d537_7ba1_b762, 0xc073_17d4_19a7_548d], // octave_-4
    [0x53d3_9c67_52da_c858, 0xbcd1_c5a8_0ab6_5b3e], // octave_-3
    [0xb4a2_4d7a_84e7_677b, 0x023f_f966_8e89_b5c4], // octave_-2
    [0xdffa_22b5_34c5_f608, 0xb9b6_7517_d366_5ca9], // octave_-1
    [0xd507_0808_6cef_4d7c, 0x6e16_51ec_c7f4_3309], // octave_0
];

/// Initial lacunarity used by `xOctaveInit` for `omin = -12..=0`.
const LACUNA_INI: [f64; 13] = [
    1.0,
    0.5,
    0.25,
    1.0 / 8.0,
    1.0 / 16.0,
    1.0 / 32.0,
    1.0 / 64.0,
    1.0 / 128.0,
    1.0 / 256.0,
    1.0 / 512.0,
    1.0 / 1024.0,
    1.0 / 2048.0,
    1.0 / 4096.0,
];

/// Initial persistence used by `xOctaveInit` for `len = 0..=9`.
const PERSIST_INI: [f64; 10] = [
    0.0,
    1.0,
    2.0 / 3.0,
    4.0 / 7.0,
    8.0 / 15.0,
    16.0 / 31.0,
    32.0 / 63.0,
    64.0 / 127.0,
    128.0 / 255.0,
    256.0 / 511.0,
];

/// A stack of Perlin octaves owned by value.
///
/// Each entry's `amplitude` and `lacunarity` are set by the initialiser
/// and remain constant for sampling. Mirrors cubiomes' `OctaveNoise`
/// (which keeps a raw `PerlinNoise *` plus a count).
#[derive(Debug, Clone)]
pub struct OctaveNoise {
    /// Owned per-octave generators, in evaluation order.
    pub octaves: Vec<PerlinNoise>,
}

impl OctaveNoise {
    /// Number of octaves in the stack.
    #[must_use]
    pub fn len(&self) -> usize {
        self.octaves.len()
    }

    /// Whether the stack is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.octaves.is_empty()
    }

    /// Initialise from a Java RNG (`octaveInit`).
    ///
    /// `omin` is the first octave index (typically negative); `len` is the
    /// number of octaves to allocate. The caller must ensure
    /// `len >= 1 && omin + len - 1 <= 0`.
    #[must_use]
    pub fn from_java(rng: &mut JavaRng, omin: i32, len: i32) -> Self {
        let end = omin + len - 1;
        assert!(len >= 1, "octave count must be at least 1");
        assert!(end <= 0, "octave range must end at <= 0");

        let mut persist = 1.0 / ((1i64 << len) as f64 - 1.0);
        let mut lacuna = (2.0_f64).powi(end);

        let mut octaves = Vec::with_capacity(len as usize);

        if end == 0 {
            let mut p = PerlinNoise::from_java(rng);
            p.amplitude = persist;
            p.lacunarity = lacuna;
            octaves.push(p);
            persist *= 2.0;
            lacuna *= 0.5;
        } else {
            // Skip the RNG forward by -end * 262 calls. The number 262 is
            // the count of next() invocations consumed by a single
            // perlinInit (3 nextDouble + 256 nextInt-driven swaps + a few
            // bookkeeping calls).
            rng.skip_n((-end) as u64 * 262);
        }

        while octaves.len() < len as usize {
            let mut p = PerlinNoise::from_java(rng);
            p.amplitude = persist;
            p.lacunarity = lacuna;
            octaves.push(p);
            persist *= 2.0;
            lacuna *= 0.5;
        }

        Self { octaves }
    }

    /// Initialise from a Java RNG with Beta-style amplitude / lacunarity
    /// multipliers (`octaveInitBeta`).
    #[must_use]
    pub fn from_java_beta(
        rng: &mut JavaRng,
        octcnt: usize,
        mut lac: f64,
        lac_mul: f64,
        mut persist: f64,
        persist_mul: f64,
    ) -> Self {
        let mut octaves = Vec::with_capacity(octcnt);
        for _ in 0..octcnt {
            let mut p = PerlinNoise::from_java(rng);
            p.amplitude = persist;
            p.lacunarity = lac;
            persist *= persist_mul;
            lac *= lac_mul;
            octaves.push(p);
        }
        Self { octaves }
    }

    /// Initialise from a Xoroshiro RNG (`xOctaveInit`), 1.18+.
    ///
    /// `amplitudes` may contain zeros; those octaves are skipped but still
    /// advance the persist / lacuna factors. `nmax` (when `Some`) caps the
    /// number of non-zero octaves actually constructed.
    #[must_use]
    pub fn from_xoroshiro(
        xr: &mut Xoroshiro,
        amplitudes: &[f64],
        omin: i32,
        nmax: Option<usize>,
    ) -> Self {
        let len = amplitudes.len();
        assert!(
            (-omin) >= 0 && ((-omin) as usize) < LACUNA_INI.len(),
            "omin out of range for LACUNA_INI"
        );
        assert!(len < PERSIST_INI.len(), "len out of range for PERSIST_INI");

        let mut lacuna = LACUNA_INI[(-omin) as usize];
        let mut persist = PERSIST_INI[len];
        let xlo = xr.next_long();
        let xhi = xr.next_long();

        let mut octaves = Vec::new();
        let mut n: usize = 0;
        for (i, &amp) in amplitudes.iter().enumerate() {
            if Some(n) == nmax {
                break;
            }
            if amp != 0.0 {
                let key = MD5_OCTAVE_N[(12 + omin) as usize + i];
                let mut pxr = Xoroshiro {
                    lo: xlo ^ key[0],
                    hi: xhi ^ key[1],
                };
                let mut p = PerlinNoise::from_xoroshiro(&mut pxr);
                p.amplitude = amp * persist;
                p.lacunarity = lacuna;
                octaves.push(p);
                n += 1;
            }
            lacuna *= 2.0;
            persist *= 0.5;
        }

        Self { octaves }
    }

    /// Sum of `amplitude * sample_perlin(x*lf, y*lf, z*lf)` across octaves.
    ///
    /// Matches `sampleOctave`.
    #[must_use]
    pub fn sample(&self, x: f64, y: f64, z: f64) -> f64 {
        let mut v = 0.0;
        for p in &self.octaves {
            let lf = p.lacunarity;
            v += p.amplitude * p.sample(x * lf, y * lf, z * lf, 0.0, 0.0);
        }
        v
    }

    /// Like `sample` but threads `yamp` / `ymin` through to each octave.
    ///
    /// When `ydefault` is true the y-input is forced to `-p.b` (preserving
    /// the y-stride-zero fast path inside `sample_perlin`). Matches
    /// `sampleOctaveAmp`.
    #[must_use]
    pub fn sample_amp(&self, x: f64, y: f64, z: f64, yamp: f64, ymin: f64, ydefault: bool) -> f64 {
        let mut v = 0.0;
        for p in &self.octaves {
            let lf = p.lacunarity;
            let ax = x * lf;
            let ay = if ydefault { -p.b } else { y * lf };
            let az = z * lf;
            v += p.amplitude * p.sample(ax, ay, az, yamp * lf, ymin * lf);
        }
        v
    }

    /// Beta-1.7 biome sampling (`sampleOctaveBeta17Biome`).
    #[must_use]
    pub fn sample_beta17_biome(&self, x: f64, z: f64) -> f64 {
        let mut v = 0.0;
        for p in &self.octaves {
            let lf = p.lacunarity;
            let ax = x * lf + p.a;
            let az = z * lf + p.b;
            v += p.amplitude * p.sample_simplex_2d(ax, az);
        }
        v
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::*;

    #[test]
    fn from_java_constructs_requested_octave_count() {
        let mut rng = JavaRng::new(42);
        let oct = OctaveNoise::from_java(&mut rng, -3, 4);
        assert_eq!(oct.len(), 4);
        // Lacunarity sequence: starts at 2^end and halves each step.
        let mut expected_lac = (2.0_f64).powi(-3 + 4 - 1);
        for p in &oct.octaves {
            assert_eq!(p.lacunarity, expected_lac);
            expected_lac *= 0.5;
        }
    }

    #[test]
    fn from_java_beta_uses_explicit_multipliers() {
        let mut rng = JavaRng::new(1);
        let oct = OctaveNoise::from_java_beta(&mut rng, 3, 1.0, 2.0, 1.0, 0.5);
        assert_eq!(oct.len(), 3);
        assert_eq!(oct.octaves[0].lacunarity, 1.0);
        assert_eq!(oct.octaves[1].lacunarity, 2.0);
        assert_eq!(oct.octaves[2].lacunarity, 4.0);
        assert_eq!(oct.octaves[0].amplitude, 1.0);
        assert_eq!(oct.octaves[1].amplitude, 0.5);
        assert_eq!(oct.octaves[2].amplitude, 0.25);
    }

    #[test]
    fn from_xoroshiro_skips_zero_amplitudes() {
        let mut xr = Xoroshiro::new(7);
        let amplitudes = [1.0, 0.0, 0.5];
        let oct = OctaveNoise::from_xoroshiro(&mut xr, &amplitudes, -3, None);
        assert_eq!(oct.len(), 2);
    }

    #[test]
    fn from_xoroshiro_respects_nmax() {
        let mut xr = Xoroshiro::new(7);
        let amplitudes = [1.0, 1.0, 1.0, 1.0];
        let oct = OctaveNoise::from_xoroshiro(&mut xr, &amplitudes, -3, Some(2));
        assert_eq!(oct.len(), 2);
    }

    #[test]
    fn sample_is_deterministic() {
        let mut rng = JavaRng::new(123);
        let oct = OctaveNoise::from_java(&mut rng, -3, 4);
        let a = oct.sample(0.5, 0.25, 0.75);
        let b = oct.sample(0.5, 0.25, 0.75);
        assert_eq!(a, b);
    }
}
