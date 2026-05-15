//! `mapSwampRiver` — pre-1.7 swamp / jungle river override.
//!
//! Bit-exact port of cubiomes' `mapSwampRiver`. Swamp cells have a
//! 1-in-6 chance of becoming `river`; `jungle` / `jungle_hills` cells
//! get 1-in-8.

#![allow(clippy::many_single_char_names)]

use crate::biome::Biome;
use crate::rng::{get_chunk_seed, mc_first_is_zero};

const SWAMP: i32 = Biome::SWAMP.id();
const JUNGLE: i32 = Biome::JUNGLE.id();
const JUNGLE_HILLS: i32 = Biome::JUNGLE_HILLS.id();
const RIVER: i32 = Biome::RIVER.id();

/// `mapSwampRiver` — parent and output rectangles coincide.
pub fn map_swamp_river(
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
        "map_swamp_river: parent slice too small"
    );
    assert!(
        out.len() >= w * h,
        "map_swamp_river: output slice too small"
    );

    for j in 0..h {
        for i in 0..w {
            let idx = i + j * w;
            let mut v = parent[idx].id();
            if v == SWAMP || v == JUNGLE || v == JUNGLE_HILLS {
                let cs = get_chunk_seed(start_seed, i as i32 + x, j as i32 + z);
                let modulus = if v == SWAMP { 6 } else { 8 };
                if mc_first_is_zero(cs, modulus) {
                    v = RIVER;
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
    fn forest_passes_through() {
        let parent = vec![Biome::FOREST; 16];
        let mut out = vec![Biome::NONE; 16];
        map_swamp_river(42, &parent, &mut out, 0, 0, 4, 4);
        for cell in &out {
            assert_eq!(*cell, Biome::FOREST);
        }
    }

    #[test]
    fn swamp_stays_or_becomes_river() {
        let parent = vec![Biome::SWAMP; 256];
        let mut out = vec![Biome::NONE; 256];
        map_swamp_river(1, &parent, &mut out, 0, 0, 16, 16);
        for cell in &out {
            assert!(*cell == Biome::SWAMP || *cell == Biome::RIVER);
        }
    }
}
