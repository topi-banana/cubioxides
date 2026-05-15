//! `mapMushroom` — sprinkles mushroom islands into open ocean.
//!
//! Bit-exact port of cubiomes' `mapMushroom`. Ocean cells whose four
//! *diagonal* neighbours are all ocean have a 1-in-100 chance of
//! becoming a `mushroom_fields` island.

#![allow(clippy::many_single_char_names)]

use crate::biome::Biome;
use crate::rng::{get_chunk_seed, mc_first_is_zero};

const OCEAN: i32 = Biome::OCEAN.id();
const MUSHROOM_FIELDS: i32 = Biome::MUSHROOM_FIELDS.id();

/// `mapMushroom` — read a `(w+2, h+2)` parent rectangle and emit a
/// `(w, h)` window.
#[allow(clippy::too_many_arguments)]
pub fn map_mushroom(
    start_seed: u64,
    parent: &[Biome],
    out: &mut [Biome],
    x: i32,
    z: i32,
    w: usize,
    h: usize,
) {
    let p_w = w + 2;
    assert!(
        parent.len() >= p_w * (h + 2),
        "map_mushroom: parent slice too small"
    );
    assert!(out.len() >= w * h, "map_mushroom: output slice too small");

    for j in 0..h {
        for i in 0..w {
            let mut v11 = parent[(i + 1) + (j + 1) * p_w].id();

            if v11 == OCEAN
                && parent[i + j * p_w].id() == OCEAN
                && parent[(i + 2) + j * p_w].id() == OCEAN
                && parent[i + (j + 2) * p_w].id() == OCEAN
                && parent[(i + 2) + (j + 2) * p_w].id() == OCEAN
            {
                let cs = get_chunk_seed(start_seed, i as i32 + x, j as i32 + z);
                if mc_first_is_zero(cs, 100) {
                    v11 = MUSHROOM_FIELDS;
                }
            }

            out[i + j * w] = Biome(v11);
        }
    }
}
