//! 1.18+ overworld biome noise generator.
//!
//! Bit-exact Rust port of cubiomes' `initBiomeNoise` +
//! `setBiomeSeed` + `sampleBiomeNoise` (the canonical "given world
//! seed + `(x, y, z)` → biome id" pipeline). Six `DoublePerlinNoise`
//! stacks are seeded from an `Xoroshiro` chain keyed on the world
//! seed, the depth channel is offset by a spline-driven term, and
//! the resulting 6-tuple is fed into
//! [`super::climate::climate_to_biome`].

#![allow(clippy::many_single_char_names)]

use crate::mc_version::MCVersion;
use crate::noise::DoublePerlinNoise;
use crate::rng::Xoroshiro;

use super::climate::climate_to_biome;
use super::spline::{SplineStack, build_overworld_spline, sample_spline};

/// Climate axis indices. Match cubiomes'
/// `enum { NP_TEMPERATURE, NP_HUMIDITY, NP_CONTINENTALNESS, NP_EROSION, NP_SHIFT = NP_DEPTH, NP_WEIRDNESS, NP_MAX }`.
pub const NP_TEMPERATURE: usize = 0;
/// Humidity axis.
pub const NP_HUMIDITY: usize = 1;
/// Continentalness axis.
pub const NP_CONTINENTALNESS: usize = 2;
/// Erosion axis.
pub const NP_EROSION: usize = 3;
/// Coordinate-shift / depth axis (shared slot — depth replaces shift
/// at sample time).
pub const NP_SHIFT: usize = 4;
/// Depth axis (alias for [`NP_SHIFT`]).
pub const NP_DEPTH: usize = NP_SHIFT;
/// Weirdness axis.
pub const NP_WEIRDNESS: usize = 5;
/// Number of climate axes.
pub const NP_MAX: usize = 6;

/// Skip the `(x, z)` Perlin "shift" displacement. Mirrors cubiomes'
/// `SAMPLE_NO_SHIFT`.
pub const SAMPLE_NO_SHIFT: u32 = 0x1;
/// Skip the spline-driven depth axis (`np[4]` stays at zero).
pub const SAMPLE_NO_DEPTH: u32 = 0x2;
/// Skip the final `climate_to_biome` call — useful for callers who
/// only want the raw climate tuple.
pub const SAMPLE_NO_BIOME: u32 = 0x4;

/// 1.18+ biome noise generator. Construct via [`Self::new`].
#[derive(Debug, Clone)]
pub struct BiomeNoise {
    /// Six per-axis `DoublePerlinNoise` stacks (temperature →
    /// weirdness).
    pub climate: [DoublePerlinNoise; NP_MAX],
    /// Continentalness / erosion / ridges / weirdness spline.
    pub spline_stack: SplineStack,
    /// Root node id within [`Self::spline_stack`].
    pub spline_root: u32,
    /// MC version (drives which decision tree
    /// [`climate_to_biome`] consults).
    pub mc: MCVersion,
}

impl BiomeNoise {
    /// Initialise + seed in a single step.
    ///
    /// `large` enables cubiomes' `LARGE_BIOMES` axes (different MD5
    /// magics + lower `omin` for the temperature / humidity /
    /// continentalness / erosion stacks).
    #[must_use]
    pub fn new(mc: MCVersion, seed: u64, large: bool) -> Self {
        let (spline_stack, spline_root) = build_overworld_spline();
        let mut pxr = Xoroshiro::new(seed);
        let xlo = pxr.next_long();
        let xhi = pxr.next_long();
        let climate: [DoublePerlinNoise; NP_MAX] =
            std::array::from_fn(|i| init_climate_seed(xlo, xhi, large, i));
        Self {
            climate,
            spline_stack,
            spline_root,
            mc,
        }
    }

    /// Re-seed the climate octaves in place without rebuilding the
    /// (`mc`-independent) spline stack. Mirrors cubiomes' standalone
    /// `setBiomeSeed`.
    pub fn re_seed(&mut self, seed: u64, large: bool) {
        let mut pxr = Xoroshiro::new(seed);
        let xlo = pxr.next_long();
        let xhi = pxr.next_long();
        for (i, slot) in self.climate.iter_mut().enumerate() {
            *slot = init_climate_seed(xlo, xhi, large, i);
        }
    }

    /// Sample the biome at `(x, y, z)`. Returns the biome id and the
    /// underlying 6-tuple `np` (units of 1/10000, sign-preserving `i64`).
    /// `sample_flags` accepts [`SAMPLE_NO_SHIFT`], [`SAMPLE_NO_DEPTH`],
    /// and [`SAMPLE_NO_BIOME`] bitwise-OR'd. When `SAMPLE_NO_BIOME` is
    /// set the returned id is `-1`.
    #[must_use]
    pub fn sample(&self, x: i32, y: i32, z: i32, sample_flags: u32) -> (i32, [i64; NP_MAX]) {
        self.sample_with_dat(x, y, z, None, sample_flags)
    }

    /// `sampleBiomeNoise(bn, np, x, y, z, dat, flags)` with the
    /// optional `dat` carry — the previous decision-tree leaf
    /// index used to emulate the order-dependent MC-241546 biome
    /// generation. Same return contract as [`Self::sample`].
    #[must_use]
    pub fn sample_with_dat(
        &self,
        x: i32,
        y: i32,
        z: i32,
        dat: Option<&mut u64>,
        sample_flags: u32,
    ) -> (i32, [i64; NP_MAX]) {
        let mut px = f64::from(x);
        let mut pz = f64::from(z);
        if sample_flags & SAMPLE_NO_SHIFT == 0 {
            px += self.climate[NP_SHIFT].sample(f64::from(x), 0.0, f64::from(z)) * 4.0;
            pz += self.climate[NP_SHIFT].sample(f64::from(z), f64::from(x), 0.0) * 4.0;
        }

        // cubiomes narrows every double sample to f32 immediately so
        // the spline + np[i] arithmetic uses f32.
        let c = self.climate[NP_CONTINENTALNESS].sample(px, 0.0, pz) as f32;
        let e = self.climate[NP_EROSION].sample(px, 0.0, pz) as f32;
        let w = self.climate[NP_WEIRDNESS].sample(px, 0.0, pz) as f32;

        let mut d: f32 = 0.0;
        if sample_flags & SAMPLE_NO_DEPTH == 0 {
            // ridges = -3 * (||w| - 2/3| - 1/3) — `np_param[2]` in cubiomes.
            let np_param = [
                c,
                e,
                -3.0_f32 * (f32::abs(f32::abs(w) - 0.666_666_7_f32) - 0.333_333_34_f32),
                w,
            ];
            // cubiomes: `double off = getSpline(bn->sp, np_param) + 0.015F;`
            // → f32 + f32 = f32, then implicit promote to f64.
            let off_f32 =
                sample_spline(&self.spline_stack, self.spline_root, &np_param) + 0.015_f32;
            let off = f64::from(off_f32);
            // d (float) = 1.0 - (y * 4) / 128.0 - 83.0/160.0 + off (all doubles, narrowed to float).
            let d_f64 = 1.0_f64 - f64::from(y * 4) / 128.0_f64 - 83.0_f64 / 160.0_f64 + off;
            d = d_f64 as f32;
        }

        let t = self.climate[NP_TEMPERATURE].sample(px, 0.0, pz) as f32;
        let h = self.climate[NP_HUMIDITY].sample(px, 0.0, pz) as f32;

        let np: [i64; NP_MAX] = [
            (10000.0_f32 * t) as i64,
            (10000.0_f32 * h) as i64,
            (10000.0_f32 * c) as i64,
            (10000.0_f32 * e) as i64,
            (10000.0_f32 * d) as i64,
            (10000.0_f32 * w) as i64,
        ];

        let id = if sample_flags & SAMPLE_NO_BIOME == 0 {
            let np_u: [u64; NP_MAX] = [
                np[0] as u64,
                np[1] as u64,
                np[2] as u64,
                np[3] as u64,
                np[4] as u64,
                np[5] as u64,
            ];
            climate_to_biome(self.mc, &np_u, dat)
        } else {
            -1
        };
        (id, np)
    }

    /// Sample the `NP_DEPTH` climate parameter at fractional `(x, z)`
    /// for `y = 0`. Bit-exact port of cubiomes' `sampleClimatePara`
    /// with `bn->nptype == NP_DEPTH`. Used by 1.18+
    /// `isViableStructureTerrain` to gate Desert/Jungle/Mansion on
    /// surface depth.
    #[must_use]
    pub fn sample_climate_para_depth(&self, x: f64, z: f64) -> f64 {
        let c = self.climate[NP_CONTINENTALNESS].sample(x, 0.0, z) as f32;
        let e = self.climate[NP_EROSION].sample(x, 0.0, z) as f32;
        let w = self.climate[NP_WEIRDNESS].sample(x, 0.0, z) as f32;
        let np_param = [
            c,
            e,
            -3.0_f32 * (f32::abs(f32::abs(w) - 0.666_666_7_f32) - 0.333_333_34_f32),
            w,
        ];
        let off_f32 = sample_spline(&self.spline_stack, self.spline_root, &np_param) + 0.015_f32;
        let off = f64::from(off_f32);
        // y=0 — cubiomes' `1.0 - (0*4)/128.0 - 83.0/160.0 + off`.
        let d_f64 = 1.0_f64 - 83.0_f64 / 160.0_f64 + off;
        f64::from(d_f64 as f32)
    }
}

/// `init_climate_seed(dpn, oct, xlo, xhi, large, nptype, nmax)` —
/// build a per-axis `DoublePerlinNoise` from the world-seed-derived
/// `(xlo, xhi)` pair. The MD5-derived XOR magics match cubiomes
/// verbatim; each axis has its own amplitude table and `omin`.
#[allow(clippy::similar_names)]
fn init_climate_seed(xlo: u64, xhi: u64, large: bool, nptype: usize) -> DoublePerlinNoise {
    let (amp, omin, lo_magic, hi_magic): (&[f64], i32, u64, u64) = match nptype {
        NP_TEMPERATURE => (
            &[1.5, 0.0, 1.0, 0.0, 0.0, 0.0],
            if large { -12 } else { -10 },
            if large {
                0x944b_0073_edf5_49db
            } else {
                0x5c7e_6b29_735f_0d7f
            },
            if large {
                0x4ff4_4347_e9d2_2b96
            } else {
                0xf7d8_6f1b_bc73_4988
            },
        ),
        NP_HUMIDITY => (
            &[1.0, 1.0, 0.0, 0.0, 0.0, 0.0],
            if large { -10 } else { -8 },
            if large {
                0x71b8_ab94_3dbd_5301
            } else {
                0x81bb_4d22_e8dc_168e
            },
            if large {
                0xbb63_ddcf_39ff_7a2b
            } else {
                0xf1c8_b4be_a163_03cd
            },
        ),
        NP_CONTINENTALNESS => (
            &[1.0, 1.0, 2.0, 2.0, 2.0, 1.0, 1.0, 1.0, 1.0],
            if large { -11 } else { -9 },
            if large {
                0x9a3f_51a1_13fc_e8dc
            } else {
                0x8388_6c9d_0ae3_a662
            },
            if large {
                0xee2d_bd15_7e5d_cdad
            } else {
                0xafa6_38a6_1b42_e8ad
            },
        ),
        NP_EROSION => (
            &[1.0, 1.0, 0.0, 1.0, 1.0],
            if large { -11 } else { -9 },
            if large {
                0x8c98_4b1f_8702_a951
            } else {
                0xd024_91e6_058f_6fd8
            },
            if large {
                0xead7_b1f9_2bae_535f
            } else {
                0x4792_512c_94c1_7a80
            },
        ),
        NP_SHIFT => (
            &[1.0, 1.0, 1.0, 0.0],
            -3,
            0x0805_18cf_6af2_5384,
            0x3f3d_fb40_a54f_ebd5,
        ),
        NP_WEIRDNESS => (
            &[1.0, 2.0, 1.0, 0.0, 0.0, 0.0],
            -7,
            0xefc8_ef4d_3610_2b34,
            0x1bee_eb32_4a0f_24ea,
        ),
        _ => panic!("unsupported nptype {nptype}"),
    };
    let mut pxr = Xoroshiro {
        lo: xlo ^ lo_magic,
        hi: xhi ^ hi_magic,
    };
    DoublePerlinNoise::from_xoroshiro(&mut pxr, amp, omin, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_deterministic() {
        let a = BiomeNoise::new(MCVersion::V1_18, 1, false);
        let b = BiomeNoise::new(MCVersion::V1_18, 1, false);
        let (id_a, np_a) = a.sample(0, 64, 0, 0);
        let (id_b, np_b) = b.sample(0, 64, 0, 0);
        assert_eq!(id_a, id_b);
        assert_eq!(np_a, np_b);
    }

    #[test]
    fn sample_flags_no_biome_yields_minus_one() {
        let bn = BiomeNoise::new(MCVersion::V1_18, 1, false);
        let (id, _) = bn.sample(0, 64, 0, SAMPLE_NO_BIOME);
        assert_eq!(id, -1);
    }
}
