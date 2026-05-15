//! `mapIsland` — remove-too-much-ocean (1.7+).
//!
//! Bit-exact port of `mapIsland` in `cubiomes/layers.c`. For each cell
//! whose centre and four cardinal neighbours are all in the
//! `Oceanic` temperature category (i.e. `ocean`), there is a 1-in-2
//! chance to flip it to `plains` so the ocean does not become solid.

#![allow(clippy::many_single_char_names)]

use crate::biome::Biome;
use crate::rng::{get_chunk_seed, mc_first_is_zero};

// `Oceanic` category equals biome ID 0, same value as `ocean`.
const OCEANIC: i32 = 0;

/// `mapIsland` — read a `(w+2, h+2)` parent rectangle starting at
/// `(x-1, z-1)` and emit a `(w, h)` window.
#[allow(clippy::too_many_arguments)]
pub fn map_island(
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
        "map_island: parent slice too small"
    );
    assert!(out.len() >= w * h, "map_island: output slice too small");

    for j in 0..h {
        for i in 0..w {
            let v11 = parent[(i + 1) + (j + 1) * p_w].id();
            out[i + j * w] = Biome(v11);

            if v11 != OCEANIC {
                continue;
            }
            if parent[(i + 1) + j * p_w].id() != OCEANIC
                || parent[(i + 2) + (j + 1) * p_w].id() != OCEANIC
                || parent[i + (j + 1) * p_w].id() != OCEANIC
                || parent[(i + 1) + (j + 2) * p_w].id() != OCEANIC
            {
                continue;
            }
            let cs = get_chunk_seed(start_seed, i as i32 + x, j as i32 + z);
            if mc_first_is_zero(cs, 2) {
                out[i + j * w] = Biome(1);
            }
        }
    }
}
