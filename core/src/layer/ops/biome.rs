//! `mapBiome` — temperature category to real biome ID assignment.
//!
//! Bit-exact port of `mapBiome` in `cubiomes/layers.c`. Pre-1.7 the
//! function uses a small `oldBiomes` table; 1.7+ it consults
//! warm/lush/cold/freezing tables plus a "rare biome" high-bit marker
//! (set by `mapSpecial`) that promotes warm to `badlands_plateau` or
//! `wooded_badlands_plateau`, lush to `jungle`, cold to
//! `giant_tree_taiga`.

#![allow(clippy::many_single_char_names)]

use crate::biome::Biome;
use crate::mc_version::MCVersion;
use crate::rng::{get_chunk_seed, mc_first_int, mc_first_is_zero};

const OCEAN: i32 = Biome::OCEAN.id();
const PLAINS: i32 = Biome::PLAINS.id();
const TAIGA: i32 = Biome::TAIGA.id();
const SNOWY_TUNDRA: i32 = Biome::SNOWY_TUNDRA.id();
const MUSHROOM_FIELDS: i32 = Biome::MUSHROOM_FIELDS.id();

// Temperature category constants (also assigned to cells by `mapSnow`).
const WARM: i32 = 1;
const LUSH: i32 = 2;
const COLD: i32 = 3;
const FREEZING: i32 = 4;

const WARM_BIOMES: [i32; 6] = [
    Biome::DESERT.id(),
    Biome::DESERT.id(),
    Biome::DESERT.id(),
    Biome::SAVANNA.id(),
    Biome::SAVANNA.id(),
    Biome::PLAINS.id(),
];
const LUSH_BIOMES: [i32; 6] = [
    Biome::FOREST.id(),
    Biome::DARK_FOREST.id(),
    Biome::MOUNTAINS.id(),
    Biome::PLAINS.id(),
    Biome::BIRCH_FOREST.id(),
    Biome::SWAMP.id(),
];
const COLD_BIOMES: [i32; 4] = [
    Biome::FOREST.id(),
    Biome::MOUNTAINS.id(),
    Biome::TAIGA.id(),
    Biome::PLAINS.id(),
];
const SNOW_BIOMES: [i32; 4] = [
    Biome::SNOWY_TUNDRA.id(),
    Biome::SNOWY_TUNDRA.id(),
    Biome::SNOWY_TUNDRA.id(),
    Biome::SNOWY_TAIGA.id(),
];
const OLD_BIOMES: [i32; 7] = [
    Biome::DESERT.id(),
    Biome::FOREST.id(),
    Biome::MOUNTAINS.id(),
    Biome::SWAMP.id(),
    Biome::PLAINS.id(),
    Biome::TAIGA.id(),
    Biome::JUNGLE.id(),
];
const OLD_BIOMES_11: [i32; 6] = [
    Biome::DESERT.id(),
    Biome::FOREST.id(),
    Biome::MOUNTAINS.id(),
    Biome::SWAMP.id(),
    Biome::PLAINS.id(),
    Biome::TAIGA.id(),
];

/// Returns `true` for the biome IDs cubiomes' `isOceanic` accepts. We
/// keep this private to `map_biome` because the wider biome category
/// system lands in M3.3+. The list matches `cubiomes/biomes.c::isOceanic`.
const fn is_oceanic(id: i32) -> bool {
    matches!(id, 0 | 10 | 24 | 44 | 45 | 46 | 47 | 48 | 49 | 50)
}

/// `mapBiome` — translate a `(w, h)` parent grid of temperature
/// categories (with optional rare-biome flag in bits 8..11) into real
/// biome IDs.
#[allow(clippy::too_many_arguments)]
pub fn map_biome(
    mc: MCVersion,
    start_seed: u64,
    parent: &[Biome],
    out: &mut [Biome],
    x: i32,
    z: i32,
    w: usize,
    h: usize,
) {
    assert!(parent.len() >= w * h, "map_biome: parent slice too small");
    assert!(out.len() >= w * h, "map_biome: output slice too small");

    let mc_le_1_6 = !mc.is_at_least(MCVersion::V1_7);
    let mc_le_1_1 = !mc.is_at_least(MCVersion::V1_2);
    let mc_le_1_2 = !mc.is_at_least(MCVersion::V1_3);

    for j in 0..h {
        for i in 0..w {
            let idx = i + j * w;
            let id_raw = parent[idx].id();
            let has_high_bit = id_raw & 0xf00 != 0;
            let id = id_raw & !0xf00;

            if mc_le_1_6 {
                if id == OCEAN || id == MUSHROOM_FIELDS {
                    out[idx] = Biome(id);
                    continue;
                }
                let cs = get_chunk_seed(start_seed, i as i32 + x, j as i32 + z);
                let mut v = if mc_le_1_1 {
                    OLD_BIOMES_11[mc_first_int(cs, 6) as usize]
                } else {
                    OLD_BIOMES[mc_first_int(cs, 7) as usize]
                };
                if id != PLAINS && (v != TAIGA || mc_le_1_2) {
                    v = SNOWY_TUNDRA;
                }
                out[idx] = Biome(v);
            } else {
                if is_oceanic(id) || id == MUSHROOM_FIELDS {
                    out[idx] = Biome(id);
                    continue;
                }
                let cs = get_chunk_seed(start_seed, i as i32 + x, j as i32 + z);
                let v = match id {
                    WARM => {
                        if has_high_bit {
                            if mc_first_is_zero(cs, 3) {
                                Biome::BADLANDS_PLATEAU.id()
                            } else {
                                Biome::WOODED_BADLANDS_PLATEAU.id()
                            }
                        } else {
                            WARM_BIOMES[mc_first_int(cs, 6) as usize]
                        }
                    }
                    LUSH => {
                        if has_high_bit {
                            Biome::JUNGLE.id()
                        } else {
                            LUSH_BIOMES[mc_first_int(cs, 6) as usize]
                        }
                    }
                    COLD => {
                        if has_high_bit {
                            Biome::GIANT_TREE_TAIGA.id()
                        } else {
                            COLD_BIOMES[mc_first_int(cs, 4) as usize]
                        }
                    }
                    FREEZING => SNOW_BIOMES[mc_first_int(cs, 4) as usize],
                    _ => MUSHROOM_FIELDS,
                };
                out[idx] = Biome(v);
            }
        }
    }
}
