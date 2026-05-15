//! `mapBamboo` — 1.14+ bamboo-jungle assignment.
//!
//! Bit-exact port of cubiomes' `mapBamboo`. Jungle cells have a 1-in-10
//! chance of becoming `bamboo_jungle`.

#![allow(clippy::many_single_char_names)]

use crate::biome::Biome;
use crate::rng::{get_chunk_seed, mc_first_is_zero};

const JUNGLE: i32 = Biome::JUNGLE.id();
const BAMBOO_JUNGLE: i32 = Biome::BAMBOO_JUNGLE.id();

/// `mapBamboo` — parent and output rectangles coincide.
pub fn map_bamboo(
    start_seed: u64,
    parent: &[Biome],
    out: &mut [Biome],
    x: i32,
    z: i32,
    w: usize,
    h: usize,
) {
    assert!(parent.len() >= w * h, "map_bamboo: parent slice too small");
    assert!(out.len() >= w * h, "map_bamboo: output slice too small");

    for j in 0..h {
        for i in 0..w {
            let idx = i + j * w;
            let mut v = parent[idx].id();
            if v == JUNGLE {
                let cs = get_chunk_seed(start_seed, i as i32 + x, j as i32 + z);
                if mc_first_is_zero(cs, 10) {
                    v = BAMBOO_JUNGLE;
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
    fn non_jungle_passes_through_unchanged() {
        let parent = vec![Biome::FOREST; 16];
        let mut out = vec![Biome::NONE; 16];
        map_bamboo(123, &parent, &mut out, 0, 0, 4, 4);
        for cell in &out {
            assert_eq!(*cell, Biome::FOREST);
        }
    }

    #[test]
    fn jungle_stays_jungle_or_becomes_bamboo() {
        let parent = vec![Biome::JUNGLE; 256];
        let mut out = vec![Biome::NONE; 256];
        map_bamboo(7, &parent, &mut out, 0, 0, 16, 16);
        for cell in &out {
            assert!(*cell == Biome::JUNGLE || *cell == Biome::BAMBOO_JUNGLE);
        }
    }
}
