//! `mapNoise` — river-initialisation layer.
//!
//! Bit-exact port of cubiomes' `mapNoise`. Non-zero parent cells get
//! `mc_first_int(cs, mod) + 2`; zero cells stay zero. The modulus
//! depends on the MC version (`2` for ≤ 1.6, `299999` otherwise) so
//! the layer needs the `MCVersion` argument.

#![allow(clippy::many_single_char_names)]

use crate::biome::Biome;
use crate::mc_version::MCVersion;
use crate::rng::{get_chunk_seed, mc_first_int};

/// `mapNoise` — parent and output rectangles coincide, no padding.
#[allow(clippy::too_many_arguments)]
pub fn map_noise(
    mc: MCVersion,
    start_seed: u64,
    parent: &[Biome],
    out: &mut [Biome],
    x: i32,
    z: i32,
    w: usize,
    h: usize,
) {
    assert!(parent.len() >= w * h, "map_noise: parent slice too small");
    assert!(out.len() >= w * h, "map_noise: output slice too small");

    let mc_le_1_6 = !mc.is_at_least(MCVersion::V1_7);
    let modulus: i32 = if mc_le_1_6 { 2 } else { 299_999 };

    for j in 0..h {
        for i in 0..w {
            let idx = i + j * w;
            let v = if parent[idx].id() > 0 {
                let cs = get_chunk_seed(start_seed, i as i32 + x, j as i32 + z);
                mc_first_int(cs, modulus) + 2
            } else {
                0
            };
            out[idx] = Biome(v);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_parent_stays_zero() {
        let parent = vec![Biome::OCEAN; 16];
        let mut out = vec![Biome::NONE; 16];
        map_noise(MCVersion::V1_7, 1234, &parent, &mut out, 0, 0, 4, 4);
        for cell in &out {
            assert_eq!(*cell, Biome::OCEAN);
        }
    }

    #[test]
    fn non_zero_parent_writes_value_in_range() {
        let parent = vec![Biome::PLAINS; 16];
        let mut out = vec![Biome::NONE; 16];
        map_noise(MCVersion::V1_7, 1234, &parent, &mut out, 0, 0, 4, 4);
        for cell in &out {
            let id = cell.id();
            assert!((2..=2 + 299_998).contains(&id), "out-of-range cell {id}");
        }
    }

    #[test]
    fn pre_1_7_modulus_is_2() {
        let parent = vec![Biome::PLAINS; 16];
        let mut out = vec![Biome::NONE; 16];
        map_noise(MCVersion::V1_6, 1234, &parent, &mut out, 0, 0, 4, 4);
        for cell in &out {
            // mc_first_int(cs, 2) + 2 ∈ {2, 3}
            assert!(matches!(cell.id(), 2 | 3));
        }
    }
}
