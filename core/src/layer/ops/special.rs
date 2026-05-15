//! `mapSpecial` — flag rare-biome variants in the high byte.
//!
//! Bit-exact port of cubiomes' `mapSpecial`. For every non-Oceanic
//! cell there is a 1-in-13 chance to set a "rare biome" marker into
//! bits 8..11 of the existing biome ID. Later layers (`mapBiomeEdge`)
//! consult this marker to choose a rare variant. Cubiomes performs
//! this in-place on the same buffer; the parent input is already the
//! `(w, h)` window so the parent and output rectangles coincide.

#![allow(clippy::many_single_char_names)]

use crate::biome::Biome;
use crate::rng::{get_chunk_seed, mc_first_int, mc_first_is_zero, mc_step_seed};

const OCEANIC: i32 = 0;

/// `mapSpecial` — operate on a `(w, h)` window (no padding).
#[allow(clippy::too_many_arguments)]
pub fn map_special(
    start_salt: u64,
    start_seed: u64,
    parent: &[Biome],
    out: &mut [Biome],
    x: i32,
    z: i32,
    w: usize,
    h: usize,
) {
    assert!(parent.len() >= w * h, "map_special: parent slice too small");
    assert!(out.len() >= w * h, "map_special: output slice too small");

    for j in 0..h {
        for i in 0..w {
            let mut v = parent[i + j * w].id();
            if v == OCEANIC {
                out[i + j * w] = Biome(v);
                continue;
            }

            let mut cs = get_chunk_seed(start_seed, i as i32 + x, j as i32 + z);
            if mc_first_is_zero(cs, 13) {
                cs = mc_step_seed(cs, start_salt);
                // (1 + mcFirstInt(cs, 15)) << 8 & 0xf00
                let flag = (((1 + mc_first_int(cs, 15)) as u32) << 8) & 0xf00;
                v = (v as u32 | flag) as i32;
            }
            out[i + j * w] = Biome(v);
        }
    }
}
