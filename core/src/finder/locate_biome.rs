//! `locateBiome` — find a random valid-biome cell within a radius.
//!
//! Bit-exact port of cubiomes' `locateBiome` in `finders.c`. The
//! 1.18+ path consults `BiomeNoise::sample_with_dat` directly with
//! the order-dependent `dat` carry (emulating MC-241546); the
//! pre-1.18 path generates a 1:4 cuboid via `Generator::gen_biomes`
//! and iterates. Both flavors apply cubiomes' streaming reservoir
//! pick with a `JavaRng`.

#![allow(clippy::many_single_char_names)]

use crate::biome::Biome;
use crate::finder::Pos;
use crate::generator::{Generator, OverworldKind, Range};
use crate::mc_version::MCVersion;
use crate::rng::JavaRng;

/// `id_matches(id, validB, validM)` — does `id` appear in either
/// of the two 64-bit biome-id bitsets? Bits 0..=127 live in
/// `valid_b`; bits 128..=191 live in `valid_m`.
#[inline]
#[must_use]
pub const fn id_matches(id: i32, valid_b: u64, valid_m: u64) -> bool {
    if id < 0 {
        return false;
    }
    if id < 128 {
        (valid_b & (1u64 << id)) != 0
    } else if id < 192 {
        (valid_m & (1u64 << (id - 128))) != 0
    } else {
        false
    }
}

/// `locateBiome(g, x, y, z, radius, validB, validM, rng, passes)` —
/// scan a `2*radius+1` cell square (at 1:4 scale, after `>> 2`)
/// centred on `(x, z)`, and return a uniformly-random
/// matching-biome cell plus the number of matches found.
///
/// `rng` is advanced in-place to match cubiomes' state mutation.
///
/// # Example
///
/// ```
/// use cubioxides::biome::Biome;
/// use cubioxides::finder::locate_biome;
/// use cubioxides::rng::JavaRng;
/// use cubioxides::{Dimension, Generator, MCVersion};
///
/// // Search for any forest cell within ±128 blocks of the origin.
/// // The `valid_b` mask sets bits 0..128; here we just allow plains
/// // (id=1) and forest (id=4).
/// let mut g = Generator::new(MCVersion::V1_16_1, 0);
/// g.apply_seed(Dimension::Overworld, 0xdead_beef);
/// let mut rng = JavaRng::new(0);
/// let valid_b = (1u64 << 1) | (1u64 << 4);
/// let (_pos, _matches) = locate_biome(&g, 0, 64, 0, 128, valid_b, 0, &mut rng);
/// ```
#[must_use]
pub fn locate_biome(
    g: &Generator,
    x: i32,
    y: i32,
    z: i32,
    radius: i32,
    valid_b: u64,
    valid_m: u64,
    rng: &mut JavaRng,
) -> (Pos, i32) {
    let mut out = Pos { x, z };
    let mut found: i32 = 0;

    if g.mc.is_at_least(MCVersion::V1_18) && matches!(g.overworld_kind, OverworldKind::Modern) {
        let xc = x >> 2;
        let zc = z >> 2;
        let r = radius >> 2;
        let bn = g
            .biome_noise
            .as_ref()
            .expect("Modern Overworld must be apply_seed'd before locate_biome");
        let mut dat: u64 = 0;
        for j in -r..=r {
            for i in -r..=r {
                let (id, _) = bn.sample_with_dat(xc + i, y, zc + j, Some(&mut dat), 0);
                if !id_matches(id, valid_b, valid_m) {
                    continue;
                }
                if found == 0 || rng.next_int(found + 1) == 0 {
                    out.x = (xc + i) * 4;
                    out.z = (zc + j) * 4;
                }
                found += 1;
            }
        }
    } else {
        // pre-1.18 / layered: pull a 1:4 cuboid via gen_biomes.
        let x1 = (x - radius) >> 2;
        let z1 = (z - radius) >> 2;
        let x2 = (x + radius) >> 2;
        let z2 = (z + radius) >> 2;
        let width = (x2 - x1 + 1) as usize;
        let height = (z2 - z1 + 1) as usize;
        let mut cache = vec![Biome::NONE; width * height];
        g.gen_biomes(
            &mut cache,
            Range {
                scale: 4,
                x: x1,
                z: z1,
                sx: width as u32,
                sz: height as u32,
                y,
                sy: 1,
            },
        );

        if g.mc.is_at_least(MCVersion::V1_13) {
            let mut j_counter: i32 = 2;
            for (i, b) in cache.iter().enumerate() {
                if !id_matches(b.id(), valid_b, valid_m) {
                    continue;
                }
                if found == 0 || rng.next_int(j_counter) == 0 {
                    out.x = (x1 + (i % width) as i32) * 4;
                    out.z = (z1 + (i / width) as i32) * 4;
                    found = 1;
                }
                j_counter += 1;
            }
            found = j_counter - 2;
        } else {
            for (i, b) in cache.iter().enumerate() {
                if !id_matches(b.id(), valid_b, valid_m) {
                    continue;
                }
                if found == 0 || rng.next_int(found + 1) == 0 {
                    out.x = (x1 + (i % width) as i32) * 4;
                    out.z = (z1 + (i / width) as i32) * 4;
                    found += 1;
                }
            }
        }
    }

    (out, found)
}
