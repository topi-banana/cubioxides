//! Beta-1.7 surface generator stack.
//!
//! Bit-exact port of cubiomes' `SurfaceNoiseBeta`,
//! `initSurfaceNoiseBeta`, `genColumnNoise`, `processColumnNoise`,
//! and `approxSurfaceBeta` from `biomenoise.{h,c}`.

#![allow(
    clippy::many_single_char_names,
    clippy::doc_markdown,
    clippy::needless_range_loop,
    clippy::items_after_statements
)]

use crate::biomenoise::beta::BiomeNoiseBeta;
use crate::noise::OctaveNoise;
use crate::rng::JavaRng;

/// 5-octave-stack Beta-1.7 surface noise generator. Mirrors
/// cubiomes' `STRUCT(SurfaceNoiseBeta)`.
#[derive(Debug, Clone)]
pub struct SurfaceNoiseBeta {
    /// Lower-envelope octaves (16).
    pub oct_min: OctaveNoise,
    /// Upper-envelope octaves (16).
    pub oct_max: OctaveNoise,
    /// Main-blend octaves (8).
    pub oct_main: OctaveNoise,
    /// Continent A octaves (10).
    pub oct_cont_a: OctaveNoise,
    /// Continent B octaves (16).
    pub oct_cont_b: OctaveNoise,
}

impl SurfaceNoiseBeta {
    /// `initSurfaceNoiseBeta(snb, seed)` — bit-exact port. The
    /// `skipNextN(s, 262 * 8)` step in between `octmain` and
    /// `octcontA` mirrors cubiomes' RNG skip.
    #[must_use]
    pub fn init(seed: u64) -> Self {
        let mut rng = JavaRng::new(seed);
        let oct_min = OctaveNoise::from_java_beta(&mut rng, 16, 684.412, 0.5, 1.0, 2.0);
        let oct_max = OctaveNoise::from_java_beta(&mut rng, 16, 684.412, 0.5, 1.0, 2.0);
        let oct_main = OctaveNoise::from_java_beta(&mut rng, 8, 684.412 / 80.0, 0.5, 1.0, 2.0);
        rng.skip_n(262 * 8);
        let oct_cont_a = OctaveNoise::from_java_beta(&mut rng, 10, 1.121, 0.5, 1.0, 2.0);
        let oct_cont_b = OctaveNoise::from_java_beta(&mut rng, 16, 200.0, 0.5, 1.0, 2.0);
        Self {
            oct_min,
            oct_max,
            oct_main,
            oct_cont_a,
            oct_cont_b,
        }
    }
}

/// Mirrors cubiomes' `STRUCT(SeaLevelColumnNoiseBeta)`.
#[derive(Debug, Clone, Copy, Default)]
pub struct SeaLevelColumnNoiseBeta {
    /// Cont-A sample.
    pub cont_a_sample: f64,
    /// Cont-B sample.
    pub cont_b_sample: f64,
    /// Min-envelope 2-value sample.
    pub min_sample: [f64; 2],
    /// Max-envelope 2-value sample.
    pub max_sample: [f64; 2],
    /// Main-blend 2-value sample.
    pub main_sample: [f64; 2],
}

/// `genColumnNoise(snb, dest, cx, cz, lacmin)` — fill `dest` with
/// the per-column noise samples needed by `processColumnNoise`.
pub fn gen_column_noise(
    snb: &SurfaceNoiseBeta,
    dest: &mut SeaLevelColumnNoiseBeta,
    cx: f64,
    cz: f64,
    lac_min: f64,
) {
    // sampleOctaveAmp(octave, x, 0, z, 0, 0, 1) — y default, no yamp.
    dest.cont_a_sample = snb.oct_cont_a.sample_amp(cx, 0.0, cz, 0.0, 0.0, true);
    dest.cont_b_sample = snb.oct_cont_b.sample_amp(cx, 0.0, cz, 0.0, 0.0, true);
    snb.oct_min
        .sample_beta17_terrain(&mut dest.min_sample, cx, cz, false, lac_min);
    snb.oct_max
        .sample_beta17_terrain(&mut dest.max_sample, cx, cz, false, lac_min);
    snb.oct_main
        .sample_beta17_terrain(&mut dest.main_sample, cx, cz, true, lac_min);
}

/// `processColumnNoise(out, src, climate)` — apply the climate
/// transform to the column samples and produce 2 final values.
///
/// Bit-exact port of cubiomes' static `processColumnNoise`.
pub fn process_column_noise(out: &mut [f64; 2], src: &SeaLevelColumnNoiseBeta, climate: &[f64; 2]) {
    let mut humi = 1.0 - climate[0] * climate[1];
    humi *= humi;
    humi *= humi;
    humi = 1.0 - humi;
    let mut cont_a = (src.cont_a_sample + 256.0) / 512.0 * humi;
    if cont_a > 1.0 {
        cont_a = 1.0;
    }
    let mut cont_b = src.cont_b_sample / 8000.0;
    if cont_b < 0.0 {
        cont_b = -cont_b * 0.3;
    }
    cont_b = cont_b * 3.0 - 2.0;
    if cont_b < 0.0 {
        cont_b /= 2.0;
        cont_b = if cont_b < -1.0 {
            -1.0 / 1.4 / 2.0
        } else {
            cont_b / 1.4 / 2.0
        };
        cont_a = 0.0;
    } else {
        if cont_b > 1.0 {
            cont_b = 1.0;
        }
        cont_b /= 8.0;
    }
    if cont_a < 0.0 {
        cont_a = 0.0;
    }
    cont_a += 0.5;
    cont_b = cont_b * 17.0 / 16.0;
    let mid = 8.5 + cont_b * 17.0;
    for k in 0..2_usize {
        let main = src.main_sample[k] / 10.0 + 1.0;
        let main = main.clamp(0.0, 1.0);
        let min_v = src.min_sample[k] / 512.0;
        let max_v = src.max_sample[k] / 512.0;
        let cell = if main < 1.0 {
            // Lerp between min and max, biased by main.
            min_v + (max_v - min_v) * main
        } else {
            max_v
        };
        let cell = cell - 8.0;
        let cell = cell + cont_a * 4.0;
        let cell = if cell > 0.0 { cell * 4.0 } else { cell };
        out[k] = cell - mid + (k as f64) * 4.0;
    }
}

/// `approxSurfaceBeta(bnb, snb, x, z)` — approximate Beta surface
/// height (cubiomes leaves a TODO for vertical sampling refinement).
#[must_use]
pub fn approx_surface_beta(bnb: &BiomeNoiseBeta, snb: &SurfaceNoiseBeta, x: i32, z: i32) -> f64 {
    let (_, t, h) = bnb.sample(x, z);
    let climate = [t, h];
    let mut col = SeaLevelColumnNoiseBeta::default();
    gen_column_noise(snb, &mut col, f64::from(x) * 0.25, f64::from(z) * 0.25, 0.0);
    let mut cols = [0.0_f64; 2];
    process_column_noise(&mut cols, &col, &climate);
    63.0 + (cols[0] * 0.125 + cols[1] * 0.875) * 0.5
}
