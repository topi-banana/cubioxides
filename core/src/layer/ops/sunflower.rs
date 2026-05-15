//! `mapSunflower` — plains-to-sunflower-plains conversion.
//!
//! Bit-exact port of cubiomes' `mapSunflower`. Plains cells have a
//! 1-in-57 chance of becoming `sunflower_plains`. cubiomes also names
//! the function `mapRareBiome` for older versions.

#![allow(clippy::many_single_char_names)]

use crate::biome::Biome;
use crate::rng::{get_chunk_seed, mc_first_is_zero};

const PLAINS: i32 = Biome::PLAINS.id();
const SUNFLOWER_PLAINS: i32 = Biome::SUNFLOWER_PLAINS.id();

/// `mapSunflower` — parent and output rectangles coincide.
pub fn map_sunflower(
    start_seed: u64,
    parent: &[Biome],
    out: &mut [Biome],
    x: i32,
    z: i32,
    w: usize,
    h: usize,
) {
    assert!(
        parent.len() >= w * h,
        "map_sunflower: parent slice too small"
    );
    assert!(out.len() >= w * h, "map_sunflower: output slice too small");

    for j in 0..h {
        for i in 0..w {
            let idx = i + j * w;
            let mut v = parent[idx].id();
            if v == PLAINS {
                let cs = get_chunk_seed(start_seed, i as i32 + x, j as i32 + z);
                if mc_first_is_zero(cs, 57) {
                    v = SUNFLOWER_PLAINS;
                }
            }
            out[idx] = Biome(v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_plains_passes_through() {
        let parent = vec![Biome::FOREST; 16];
        let mut out = vec![Biome::NONE; 16];
        map_sunflower(42, &parent, &mut out, 0, 0, 4, 4);
        for cell in &out {
            assert_eq!(*cell, Biome::FOREST);
        }
    }

    #[test]
    fn plains_stays_or_becomes_sunflower() {
        let parent = vec![Biome::PLAINS; 256];
        let mut out = vec![Biome::NONE; 256];
        map_sunflower(1, &parent, &mut out, 0, 0, 16, 16);
        for cell in &out {
            assert!(*cell == Biome::PLAINS || *cell == Biome::SUNFLOWER_PLAINS);
        }
    }
}
