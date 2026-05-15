//! `mapSnow16` and `mapSnow` — pre-1.7 snow assignment and 1.7+
//! temperature-category assignment.

#![allow(clippy::many_single_char_names)]

use crate::biome::Biome;
use crate::rng::{get_chunk_seed, mc_first_int, mc_first_is_zero};

const OCEAN: i32 = Biome::OCEAN.id();
const PLAINS: i32 = Biome::PLAINS.id();
const SNOWY_TUNDRA: i32 = Biome::SNOWY_TUNDRA.id();

// BiomeTempCategory values (cubiomes/layers.h):
const WARM: i32 = 1;
const COLD: i32 = 3;
const FREEZING: i32 = 4;

/// `mapSnow16` — pre-1.7 snow assignment: non-ocean cells become either
/// `snowy_tundra` (1/5) or `plains` (4/5).
#[allow(clippy::too_many_arguments)]
pub fn map_snow16(
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
        "map_snow16: parent slice too small"
    );
    assert!(out.len() >= w * h, "map_snow16: output slice too small");

    for j in 0..h {
        for i in 0..w {
            let parent_id = parent[(i + 1) + (j + 1) * p_w].id();
            let v11 = if parent_id == OCEAN {
                parent_id
            } else {
                let cs = get_chunk_seed(start_seed, i as i32 + x, j as i32 + z);
                if mc_first_is_zero(cs, 5) {
                    SNOWY_TUNDRA
                } else {
                    PLAINS
                }
            };
            out[i + j * w] = Biome(v11);
        }
    }
}

/// `mapSnow` — 1.7+ temperature-category assignment.
///
/// Cells that are *not* shallow ocean roll for one of `Freezing`,
/// `Cold`, `Warm` with probabilities 1/6, 1/6, 4/6.
#[allow(clippy::too_many_arguments)]
pub fn map_snow(
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
        "map_snow: parent slice too small"
    );
    assert!(out.len() >= w * h, "map_snow: output slice too small");

    for j in 0..h {
        for i in 0..w {
            let parent_id = parent[(i + 1) + (j + 1) * p_w].id();
            let v11 = if Biome::is_shallow_ocean_id(parent_id) {
                parent_id
            } else {
                let cs = get_chunk_seed(start_seed, i as i32 + x, j as i32 + z);
                let r = mc_first_int(cs, 6);
                if r == 0 {
                    FREEZING
                } else if r <= 1 {
                    COLD
                } else {
                    WARM
                }
            };
            out[i + j * w] = Biome(v11);
        }
    }
}
