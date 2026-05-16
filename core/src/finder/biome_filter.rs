//! `BiomeFilter` + `setupBiomeFilter` — pre-compute per-layer biome
//! bitmasks for the required / excluded / match-any sets, so that
//! `checkForBiomes` can short-circuit a chunked layer-DAG walk.
//!
//! Bit-exact port of cubiomes' `setupBiomeFilter` from `finders.c`.
//! The bit positions and layer-association of each mask field are
//! load-bearing: changing any of them silently breaks `checkForBiomes`.

#![allow(clippy::too_many_lines, clippy::cognitive_complexity, missing_docs)]

use crate::biome::Biome;
use crate::finder::gen_potential::gen_potential;
use crate::layer::stack::LayerId;
use crate::mc_version::MCVersion;

// Mirrors cubiomes' anonymous `BiomeTempCategory`:
// `{Oceanic, Warm, Lush, Cold, Freezing, Special}`. `Warm+Special`
// etc. become bit positions in `tempsToFind` / `tempsToExcl`.
const OCEANIC: u32 = 0;
const WARM: u32 = 1;
const LUSH: u32 = 2;
const COLD: u32 = 3;
const FREEZING: u32 = 4;
const SPECIAL: u32 = 5;

// Raw biome IDs (subset that this function name-checks).
const OCEAN: i32 = 0;
const PLAINS: i32 = 1;
const DESERT: i32 = 2;
const MOUNTAINS: i32 = 3;
const FOREST: i32 = 4;
const TAIGA: i32 = 5;
const SWAMP: i32 = 6;
const RIVER: i32 = 7;
const FROZEN_OCEAN: i32 = 10;
const FROZEN_RIVER: i32 = 11;
const SNOWY_TUNDRA: i32 = 12;
const SNOWY_MOUNTAINS: i32 = 13;
const MUSHROOM_FIELDS: i32 = 14;
const MUSHROOM_FIELD_SHORE: i32 = 15;
const BEACH: i32 = 16;
const DESERT_HILLS: i32 = 17;
const WOODED_HILLS: i32 = 18;
const TAIGA_HILLS: i32 = 19;
const JUNGLE: i32 = 21;
const JUNGLE_HILLS: i32 = 22;
const JUNGLE_EDGE: i32 = 23;
const DEEP_OCEAN: i32 = 24;
const STONE_SHORE: i32 = 25;
const SNOWY_BEACH: i32 = 26;
const BIRCH_FOREST: i32 = 27;
const BIRCH_FOREST_HILLS: i32 = 28;
const DARK_FOREST: i32 = 29;
const SNOWY_TAIGA: i32 = 30;
const SNOWY_TAIGA_HILLS: i32 = 31;
const GIANT_TREE_TAIGA: i32 = 32;
const GIANT_TREE_TAIGA_HILLS: i32 = 33;
const WOODED_MOUNTAINS: i32 = 34;
const SAVANNA: i32 = 35;
const SAVANNA_PLATEAU: i32 = 36;
const BADLANDS: i32 = 37;
const WOODED_BADLANDS_PLATEAU: i32 = 38;
const BADLANDS_PLATEAU: i32 = 39;
const WARM_OCEAN: i32 = 44;
const LUKEWARM_OCEAN: i32 = 45;
const COLD_OCEAN: i32 = 46;
const DEEP_WARM_OCEAN: i32 = 47;
const DEEP_FROZEN_OCEAN: i32 = 50;
const SUNFLOWER_PLAINS: i32 = 129;
const DESERT_LAKES: i32 = 130;
const GRAVELLY_MOUNTAINS: i32 = 131;
const FLOWER_FOREST: i32 = 132;
const TAIGA_MOUNTAINS: i32 = 133;
const SWAMP_HILLS: i32 = 134;
const ICE_SPIKES: i32 = 140;
const MODIFIED_JUNGLE: i32 = 149;
const MODIFIED_JUNGLE_EDGE: i32 = 151;
const TALL_BIRCH_FOREST: i32 = 155;
const TALL_BIRCH_HILLS: i32 = 156;
const DARK_FOREST_HILLS: i32 = 157;
const SNOWY_TAIGA_MOUNTAINS: i32 = 158;
const GIANT_SPRUCE_TAIGA: i32 = 160;
const GIANT_SPRUCE_TAIGA_HILLS: i32 = 161;
const MODIFIED_GRAVELLY_MOUNTAINS: i32 = 162;
const SHATTERED_SAVANNA: i32 = 163;
const SHATTERED_SAVANNA_PLATEAU: i32 = 164;
const ERODED_BADLANDS: i32 = 165;
const MODIFIED_WOODED_BADLANDS_PLATEAU: i32 = 166;
const MODIFIED_BADLANDS_PLATEAU: i32 = 167;
const BAMBOO_JUNGLE: i32 = 168;
const BAMBOO_JUNGLE_HILLS: i32 = 169;

const FORCE_OCEAN_VARIANTS: u32 = 0x4;

/// Per-layer biome bitmasks compiled from a target query
/// (`required` / `excluded` / `matchany` lists). Layout matches
/// cubiomes' `BiomeFilter` struct exactly.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct BiomeFilter {
    // Required-side: bits represent biomes that must appear at each layer.
    /// Special (1:1024) temperature categories.
    pub temps_to_find: u64,
    /// `OceanTemp` (1:256) — only set for ocean queries.
    pub otemp_to_find: u64,
    /// Biome (1:256) base-biome bits.
    pub major_to_find: u64,
    /// Edge (1:64) — bamboo lives at bit `bamboo & 0x3f`.
    pub edges_to_find: u64,
    /// `RareBiome` (1:64) bits for IDs < 64.
    pub rares_to_find: u64,
    /// `RareBiome` (1:64) bits for IDs 128..192 (mapped to bit `id - 128`).
    pub rares_to_find_m: u64,
    /// Shore (1:16) bits for IDs < 64.
    pub shore_to_find: u64,
    /// Shore (1:16) bits for IDs 128..192.
    pub shore_to_find_m: u64,
    /// Mix (1:4) bits for IDs < 64.
    pub river_to_find: u64,
    /// Mix (1:4) bits for IDs 128..192.
    pub river_to_find_m: u64,
    /// All required ocean types.
    pub ocean_to_find: u64,
    /// Number of `Warm/Lush/Cold + Special` flags set in `temps_to_find`.
    pub special_cnt: i32,
    /// Flags carried verbatim from the user (e.g. `FORCE_OCEAN_VARIANTS`).
    pub flags: u32,
    // Excluded-side: biomes that, if encountered, abort the search.
    pub temps_to_excl: u64,
    pub major_to_excl: u64,
    pub edges_to_excl: u64,
    pub rares_to_excl: u64,
    pub rares_to_excl_m: u64,
    pub shore_to_excl: u64,
    pub shore_to_excl_m: u64,
    pub river_to_excl: u64,
    pub river_to_excl_m: u64,
    pub biome_to_excl: u64,
    pub biome_to_excl_m: u64,
    /// Final biome-id required mask (post-shore mixing).
    pub biome_to_find: u64,
    pub biome_to_find_m: u64,
    /// Final biome-id match-any mask (the union of every matchany entry).
    pub biome_to_pick: u64,
    pub biome_to_pick_m: u64,
}

/// Bit-exact port of cubiomes' `setupBiomeFilter`.
///
/// Builds a [`BiomeFilter`] capturing the per-layer bitmasks needed
/// by `checkForBiomes` to short-circuit a search. `required` /
/// `excluded` / `matchany` are interpreted as biome ID lists.
///
/// **IDs outside `[0, 64) ∪ [128, 192)` cause cubiomes to abort the
/// process via `exit(-1)`. We return `None` in that case instead.**
#[must_use]
pub fn setup_biome_filter(
    mc: MCVersion,
    flags: u32,
    required: &[i32],
    excluded: &[i32],
    matchany: &[i32],
) -> Option<BiomeFilter> {
    let mut bf = BiomeFilter {
        flags,
        ..BiomeFilter::default()
    };

    // matchany: AND-fold the per-id sub-filter into the running filter.
    for (i, &id) in matchany.iter().enumerate() {
        if id < 128 {
            bf.biome_to_pick |= 1_u64 << id;
        } else {
            bf.biome_to_pick_m |= 1_u64 << (id - 128);
        }
        let ibf = setup_biome_filter(mc, 0, &[id], &[], &[])?;
        if i == 0 {
            bf.temps_to_find = ibf.temps_to_find;
            bf.otemp_to_find = ibf.otemp_to_find;
            bf.major_to_find = ibf.major_to_find;
            bf.edges_to_find = ibf.edges_to_find;
            bf.rares_to_find = ibf.rares_to_find;
            bf.rares_to_find_m = ibf.rares_to_find_m;
            bf.shore_to_find = ibf.shore_to_find;
            bf.shore_to_find_m = ibf.shore_to_find_m;
            bf.river_to_find = ibf.river_to_find;
            bf.river_to_find_m = ibf.river_to_find_m;
            bf.ocean_to_find = ibf.ocean_to_find;
        } else {
            bf.temps_to_find &= ibf.temps_to_find;
            bf.otemp_to_find &= ibf.otemp_to_find;
            bf.major_to_find &= ibf.major_to_find;
            bf.edges_to_find &= ibf.edges_to_find;
            bf.rares_to_find &= ibf.rares_to_find;
            bf.rares_to_find_m &= ibf.rares_to_find_m;
            bf.shore_to_find &= ibf.shore_to_find;
            bf.shore_to_find_m &= ibf.shore_to_find_m;
            bf.river_to_find &= ibf.river_to_find;
            bf.river_to_find_m &= ibf.river_to_find_m;
            bf.ocean_to_find &= ibf.ocean_to_find;
        }
    }

    // excluded: stamp into biome_to_excl{,M}. Then derive per-layer
    // excludes via genPotential for the 1.7+ layered path.
    for &id in excluded {
        if (id & !0xbf) != 0 {
            return None; // cubiomes exits, we err out.
        }
        if id < 128 {
            bf.biome_to_excl |= 1_u64 << id;
        } else {
            bf.biome_to_excl_m |= 1_u64 << (id - 128);
        }
    }
    if !excluded.is_empty() && mc.is_at_least(MCVersion::V1_7) {
        // tempsToExcl loop: cubiomes iterates j = Oceanic..=Freezing+Special.
        for j in OCEANIC..=(FREEZING + SPECIAL) {
            let mut b: u64 = 0;
            let mut m: u64 = 0;
            let temp: i32 = if j <= FREEZING {
                j as i32
            } else {
                ((j - SPECIAL) as i32) | 0xf00
            };
            gen_potential(mc, flags, LayerId::Special1024, temp, &mut b, &mut m);
            if (bf.biome_to_excl & b) != 0 || (bf.biome_to_excl_m & m) != 0 {
                bf.temps_to_excl |= 1_u64 << j;
            }
        }
        for j in 0..256_i32 {
            if !Biome::is_overworld_id(mc, j) {
                continue;
            }
            if j < 128 {
                let mut b: u64 = 0;
                let mut m: u64 = 0;
                gen_potential(mc, flags, LayerId::Biome256, j, &mut b, &mut m);
                if (!bf.biome_to_excl & b) != 0 || (!bf.biome_to_excl_m & m) != 0 {
                    bf.major_to_excl |= 1_u64 << j;
                }
            }
            let mut b: u64 = 0;
            let mut m: u64 = 0;
            gen_potential(mc, flags, LayerId::BiomeEdge64, j, &mut b, &mut m);
            if (!bf.biome_to_excl & b) != 0 || (!bf.biome_to_excl_m & m) != 0 {
                if j < 128 {
                    bf.edges_to_excl |= 1_u64 << j;
                } else {
                    // bamboo_jungle maps onto bit `id & 0x3f` (cubiomes).
                    bf.edges_to_excl |= 1_u64 << (j - 128);
                }
            }
            let mut b: u64 = 0;
            let mut m: u64 = 0;
            gen_potential(mc, flags, LayerId::Sunflower64, j, &mut b, &mut m);
            if (!bf.biome_to_excl & b) != 0 || (!bf.biome_to_excl_m & m) != 0 {
                if j < 128 {
                    bf.rares_to_excl |= 1_u64 << j;
                } else {
                    bf.rares_to_excl_m |= 1_u64 << (j - 128);
                }
            }
            let mut b: u64 = 0;
            let mut m: u64 = 0;
            gen_potential(mc, flags, LayerId::Shore16, j, &mut b, &mut m);
            if (!bf.biome_to_excl & b) != 0 || (!bf.biome_to_excl_m & m) != 0 {
                if j < 128 {
                    bf.shore_to_excl |= 1_u64 << j;
                } else {
                    bf.shore_to_excl_m |= 1_u64 << (j - 128);
                }
            }
            let mut b: u64 = 0;
            let mut m: u64 = 0;
            gen_potential(mc, flags, LayerId::RiverMix4, j, &mut b, &mut m);
            if (!bf.biome_to_excl & b) != 0 || (!bf.biome_to_excl_m & m) != 0 {
                if j < 128 {
                    bf.river_to_excl |= 1_u64 << j;
                } else {
                    bf.river_to_excl_m |= 1_u64 << (j - 128);
                }
            }
        }
    }

    // required: per-id stamp into find masks. Long switch by biome ID.
    for &id in required {
        if (id & !0xbf) != 0 {
            return None;
        }

        match id {
            MUSHROOM_FIELDS => {
                bf.rares_to_find |= 1_u64 << MUSHROOM_FIELDS;
                // fall through to MUSHROOM_FIELD_SHORE
                bf.temps_to_find |= 1_u64 << OCEANIC;
                bf.major_to_find |= 1_u64 << MUSHROOM_FIELDS;
                bf.river_to_find |= 1_u64 << id;
            }
            MUSHROOM_FIELD_SHORE => {
                bf.temps_to_find |= 1_u64 << OCEANIC;
                bf.major_to_find |= 1_u64 << MUSHROOM_FIELDS;
                bf.river_to_find |= 1_u64 << id;
            }
            BADLANDS_PLATEAU
            | WOODED_BADLANDS_PLATEAU
            | BADLANDS
            | ERODED_BADLANDS
            | MODIFIED_BADLANDS_PLATEAU
            | MODIFIED_WOODED_BADLANDS_PLATEAU => {
                bf.temps_to_find |= 1_u64 << (WARM + SPECIAL);
                if id == BADLANDS_PLATEAU || id == MODIFIED_BADLANDS_PLATEAU {
                    bf.major_to_find |= 1_u64 << BADLANDS_PLATEAU;
                }
                if id == WOODED_BADLANDS_PLATEAU || id == MODIFIED_WOODED_BADLANDS_PLATEAU {
                    bf.major_to_find |= 1_u64 << WOODED_BADLANDS_PLATEAU;
                }
                if id < 128 {
                    bf.rares_to_find |= 1_u64 << id;
                    bf.river_to_find |= 1_u64 << id;
                } else {
                    bf.rares_to_find_m |= 1_u64 << (id - 128);
                    bf.river_to_find_m |= 1_u64 << (id - 128);
                }
            }
            JUNGLE | JUNGLE_EDGE | JUNGLE_HILLS | MODIFIED_JUNGLE | MODIFIED_JUNGLE_EDGE
            | BAMBOO_JUNGLE | BAMBOO_JUNGLE_HILLS => {
                bf.temps_to_find |= 1_u64 << (LUSH + SPECIAL);
                bf.major_to_find |= 1_u64 << JUNGLE;
                if id == BAMBOO_JUNGLE || id == BAMBOO_JUNGLE_HILLS {
                    bf.edges_to_find |= 1_u64 << (BAMBOO_JUNGLE & 0x3f);
                    bf.rares_to_find_m |= 1_u64 << (id - 128);
                    bf.river_to_find_m |= 1_u64 << (id - 128);
                } else if id == JUNGLE_EDGE {
                    bf.river_to_find |= 1_u64 << JUNGLE_EDGE;
                } else {
                    if id == MODIFIED_JUNGLE_EDGE {
                        bf.edges_to_find |= 1_u64 << JUNGLE_EDGE;
                    } else {
                        bf.edges_to_find |= 1_u64 << JUNGLE;
                    }
                    if id < 128 {
                        bf.rares_to_find |= 1_u64 << id;
                        bf.river_to_find |= 1_u64 << id;
                    } else {
                        bf.rares_to_find_m |= 1_u64 << (id - 128);
                        bf.river_to_find_m |= 1_u64 << (id - 128);
                    }
                }
            }
            GIANT_TREE_TAIGA
            | GIANT_TREE_TAIGA_HILLS
            | GIANT_SPRUCE_TAIGA
            | GIANT_SPRUCE_TAIGA_HILLS => {
                bf.temps_to_find |= 1_u64 << (COLD + SPECIAL);
                bf.major_to_find |= 1_u64 << GIANT_TREE_TAIGA;
                bf.edges_to_find |= 1_u64 << GIANT_TREE_TAIGA;
                if id < 128 {
                    bf.rares_to_find |= 1_u64 << id;
                    bf.river_to_find |= 1_u64 << id;
                } else {
                    bf.rares_to_find_m |= 1_u64 << (id - 128);
                    bf.river_to_find_m |= 1_u64 << (id - 128);
                }
            }
            SAVANNA
            | SAVANNA_PLATEAU
            | SHATTERED_SAVANNA
            | SHATTERED_SAVANNA_PLATEAU
            | DESERT_HILLS
            | DESERT_LAKES => {
                bf.temps_to_find |= 1_u64 << WARM;
                if id == DESERT_HILLS || id == DESERT_LAKES {
                    bf.major_to_find |= 1_u64 << DESERT;
                    bf.edges_to_find |= 1_u64 << DESERT;
                } else {
                    bf.major_to_find |= 1_u64 << SAVANNA;
                    bf.edges_to_find |= 1_u64 << SAVANNA;
                }
                if id < 128 {
                    bf.rares_to_find |= 1_u64 << id;
                    bf.river_to_find |= 1_u64 << id;
                } else {
                    bf.rares_to_find_m |= 1_u64 << (id - 128);
                    bf.river_to_find_m |= 1_u64 << (id - 128);
                }
            }
            DARK_FOREST | DARK_FOREST_HILLS | BIRCH_FOREST | BIRCH_FOREST_HILLS
            | TALL_BIRCH_FOREST | TALL_BIRCH_HILLS | SWAMP | SWAMP_HILLS => {
                bf.temps_to_find |= 1_u64 << LUSH;
                if id == DARK_FOREST || id == DARK_FOREST_HILLS {
                    bf.major_to_find |= 1_u64 << DARK_FOREST;
                    bf.edges_to_find |= 1_u64 << DARK_FOREST;
                } else if id == BIRCH_FOREST
                    || id == BIRCH_FOREST_HILLS
                    || id == TALL_BIRCH_FOREST
                    || id == TALL_BIRCH_HILLS
                {
                    bf.major_to_find |= 1_u64 << BIRCH_FOREST;
                    bf.edges_to_find |= 1_u64 << BIRCH_FOREST;
                } else if id == SWAMP || id == SWAMP_HILLS {
                    bf.major_to_find |= 1_u64 << SWAMP;
                    bf.edges_to_find |= 1_u64 << SWAMP;
                }
                if id < 128 {
                    bf.rares_to_find |= 1_u64 << id;
                    bf.river_to_find |= 1_u64 << id;
                } else {
                    bf.rares_to_find_m |= 1_u64 << (id - 128);
                    bf.river_to_find_m |= 1_u64 << (id - 128);
                }
            }
            SNOWY_TAIGA
            | SNOWY_TAIGA_HILLS
            | SNOWY_TAIGA_MOUNTAINS
            | SNOWY_TUNDRA
            | SNOWY_MOUNTAINS
            | ICE_SPIKES
            | FROZEN_RIVER => {
                bf.temps_to_find |= 1_u64 << FREEZING;
                if id == SNOWY_TAIGA || id == SNOWY_TAIGA_HILLS || id == SNOWY_TAIGA_MOUNTAINS {
                    bf.edges_to_find |= 1_u64 << SNOWY_TAIGA;
                } else {
                    bf.edges_to_find |= 1_u64 << SNOWY_TUNDRA;
                }
                if id == FROZEN_RIVER {
                    bf.rares_to_find |= 1_u64 << SNOWY_TUNDRA;
                    bf.river_to_find |= 1_u64 << id;
                } else if id < 128 {
                    bf.rares_to_find |= 1_u64 << id;
                    bf.river_to_find |= 1_u64 << id;
                } else {
                    bf.rares_to_find_m |= 1_u64 << (id - 128);
                    bf.river_to_find_m |= 1_u64 << (id - 128);
                }
            }
            SNOWY_BEACH => {
                bf.temps_to_find |= 1_u64 << FREEZING;
                bf.river_to_find |= 1_u64 << id;
            }
            BEACH | STONE_SHORE | DESERT => {
                bf.river_to_find |= 1_u64 << id;
            }
            MOUNTAINS => {
                bf.major_to_find |= 1_u64 << MOUNTAINS;
                bf.rares_to_find |= 1_u64 << id;
                bf.river_to_find |= 1_u64 << id;
            }
            WOODED_MOUNTAINS | PLAINS | FOREST | WOODED_HILLS => {
                bf.rares_to_find |= 1_u64 << id;
                bf.river_to_find |= 1_u64 << id;
            }
            GRAVELLY_MOUNTAINS => {
                bf.major_to_find |= 1_u64 << MOUNTAINS;
                bf.rares_to_find_m |= 1_u64 << (id - 128);
                bf.river_to_find_m |= 1_u64 << (id - 128);
            }
            SUNFLOWER_PLAINS | MODIFIED_GRAVELLY_MOUNTAINS | FLOWER_FOREST => {
                bf.rares_to_find_m |= 1_u64 << (id - 128);
                bf.river_to_find_m |= 1_u64 << (id - 128);
            }
            TAIGA | TAIGA_HILLS => {
                bf.edges_to_find |= 1_u64 << TAIGA;
                bf.rares_to_find |= 1_u64 << id;
                bf.river_to_find |= 1_u64 << id;
            }
            TAIGA_MOUNTAINS => {
                bf.edges_to_find |= 1_u64 << TAIGA;
                bf.rares_to_find_m |= 1_u64 << (id - 128);
                bf.river_to_find_m |= 1_u64 << (id - 128);
            }
            _ => {
                if Biome::is_oceanic_id(id) {
                    bf.temps_to_find |= 1_u64 << OCEANIC;
                    bf.ocean_to_find |= 1_u64 << id;
                    if Biome::is_shallow_ocean_id(id) {
                        if id != LUKEWARM_OCEAN && id != COLD_OCEAN {
                            bf.otemp_to_find |= 1_u64 << id;
                        }
                    } else {
                        if id == DEEP_WARM_OCEAN {
                            bf.otemp_to_find |= 1_u64 << WARM_OCEAN;
                        } else if id == DEEP_OCEAN {
                            bf.otemp_to_find |= 1_u64 << OCEAN;
                        } else if id == DEEP_FROZEN_OCEAN {
                            bf.otemp_to_find |= 1_u64 << FROZEN_OCEAN;
                        }
                        if flags & FORCE_OCEAN_VARIANTS == 0 {
                            bf.rares_to_find |= 1_u64 << DEEP_OCEAN;
                            bf.river_to_find |= 1_u64 << DEEP_OCEAN;
                        }
                    }
                } else if id < 64 {
                    bf.river_to_find |= 1_u64 << id;
                } else {
                    bf.river_to_find_m |= 1_u64 << (id - 128);
                }
            }
        }
    }

    // Post-processing: derive biome_to_find / biome_to_find_m /
    // shore_to_find{,M} / special_cnt from river/ocean masks.
    bf.biome_to_find = bf.river_to_find;
    bf.biome_to_find &= !((1_u64 << OCEAN) | (1_u64 << DEEP_OCEAN));
    bf.biome_to_find |= bf.ocean_to_find;
    bf.biome_to_find_m = bf.river_to_find_m;

    bf.shore_to_find = bf.river_to_find;
    bf.shore_to_find &= !((1_u64 << RIVER) | (1_u64 << FROZEN_RIVER));
    bf.shore_to_find_m = bf.river_to_find_m;

    bf.special_cnt = 0;
    if bf.temps_to_find & (1_u64 << (WARM + SPECIAL)) != 0 {
        bf.special_cnt += 1;
    }
    if bf.temps_to_find & (1_u64 << (LUSH + SPECIAL)) != 0 {
        bf.special_cnt += 1;
    }
    if bf.temps_to_find & (1_u64 << (COLD + SPECIAL)) != 0 {
        bf.special_cnt += 1;
    }

    Some(bf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_filter_is_zero() {
        let bf = setup_biome_filter(MCVersion::V1_17, 0, &[], &[], &[]).unwrap();
        assert_eq!(bf.biome_to_find, 0);
        assert_eq!(bf.river_to_find, 0);
        assert_eq!(bf.special_cnt, 0);
    }

    #[test]
    fn required_mushroom_stamps_oceanic() {
        let bf = setup_biome_filter(MCVersion::V1_17, 0, &[MUSHROOM_FIELDS], &[], &[]).unwrap();
        assert_eq!(bf.temps_to_find, 1 << OCEANIC);
        assert_eq!(bf.major_to_find, 1 << MUSHROOM_FIELDS);
        assert!(bf.rares_to_find & (1 << MUSHROOM_FIELDS) != 0);
        assert!(bf.river_to_find & (1 << MUSHROOM_FIELDS) != 0);
    }

    #[test]
    fn out_of_range_id_returns_none() {
        // ID 100 is in the unsupported gap [64, 128).
        assert!(setup_biome_filter(MCVersion::V1_17, 0, &[100], &[], &[]).is_none());
    }
}
