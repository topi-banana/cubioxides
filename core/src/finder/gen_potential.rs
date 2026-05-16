//! `genPotential` — recursive layer-DAG descent that ORs every
//! biome ID reachable from `(layer, id)` into a 128-bit mask
//! split across `mL` (IDs 0–127) and `mM` (IDs 128–255). Bit-exact
//! port of cubiomes' `_genPotential` from `finders.c`.
//!
//! Used by `setupBiomeFilter` / `checkForBiomes` to pre-compute the
//! reachable-biome envelope of a target query.

#![allow(clippy::too_many_lines, clippy::cognitive_complexity)]

use crate::biome::Biome;
use crate::finder::can_biome_generate::can_biome_generate;
use crate::layer::stack::LayerId;
use crate::mc_version::MCVersion;

// Raw biome IDs from cubiomes' `biomes.h`. Keep the C names so the
// port remains diff-able against `_genPotential`.
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
const MOUNTAIN_EDGE: i32 = 20;
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
const DEEP_LUKEWARM_OCEAN: i32 = 48;
const DEEP_COLD_OCEAN: i32 = 49;
const DEEP_FROZEN_OCEAN: i32 = 50;
const SUNFLOWER_PLAINS: i32 = 129;
const BAMBOO_JUNGLE: i32 = 168;
const BAMBOO_JUNGLE_HILLS: i32 = 169;

// `BiomeTempCategory` literals — matches cubiomes' anonymous enum
// `{Oceanic, Warm, Lush, Cold, Freezing, Special}`.
const OCEANIC: i32 = 0;
const FREEZING: i32 = 4;

/// OR every reachable biome ID into the (mL, mM) mask pair.
///
/// `layer` is the layer at which the seed pair `(layer, id)` is
/// observed; the function then walks forward through child layers
/// until reaching the voronoi cell, OR-ing every final biome ID
/// it touches. Bits 0..=127 land in `mL`, bits 128..=255 in `mM`
/// (offset by 128, matching cubiomes).
///
/// # Example
///
/// ```
/// use cubioxides::MCVersion;
/// use cubioxides::finder::gen_potential;
/// use cubioxides::layer::LayerId;
///
/// // Walk forward from L_BIOME_256 with the plains id (1). The
/// // reachable-biome envelope feeds setup_biome_filter so callers
/// // know which final biomes can ever appear downstream — useful
/// // for pre-screening masks before kicking off a full scan.
/// let mut m_l = 0u64;
/// let mut m_m = 0u64;
/// gen_potential(MCVersion::V1_16_1, 0, LayerId::Biome256, 1, &mut m_l, &mut m_m);
/// // Plains itself is one of the reachable terminal biomes.
/// assert_ne!(m_l & (1u64 << 1), 0);
/// ```
pub fn gen_potential(
    mc: MCVersion,
    flags: u32,
    layer: LayerId,
    id: i32,
    m_l: &mut u64,
    m_m: &mut u64,
) {
    gen_potential_inner(mc, flags, layer, id, m_l, m_m);
}

fn or_bit(m_l: &mut u64, m_m: &mut u64, id: i32) {
    if id < 128 {
        *m_l |= 1_u64 << id;
    } else {
        *m_m |= 1_u64 << (id - 128);
    }
}

fn gen_potential_inner(
    mc: MCVersion,
    flags: u32,
    layer: LayerId,
    id: i32,
    m_l: &mut u64,
    m_m: &mut u64,
) {
    use LayerId as L;
    // filter out bad biomes
    if (layer as u32) >= (LayerId::Biome256 as u32) && !can_biome_generate(layer, mc, flags, id) {
        return;
    }

    match layer {
        L::Special1024 => {
            if !mc.is_at_least(MCVersion::V1_7) {
                return;
            }
            if id == OCEANIC {
                gen_potential_inner(mc, flags, L::Mushroom256, MUSHROOM_FIELDS, m_l, m_m);
            }
            let base = id & !0xf00;
            if (OCEANIC..=FREEZING).contains(&base) {
                gen_potential_inner(mc, flags, L::Mushroom256, id, m_l, m_m);
            }
        }
        L::Mushroom256 => {
            if mc.is_at_least(MCVersion::V1_7) {
                if id == OCEANIC {
                    gen_potential_inner(mc, flags, L::DeepOcean256, DEEP_OCEAN, m_l, m_m);
                }
                if id == MUSHROOM_FIELDS {
                    gen_potential_inner(mc, flags, L::DeepOcean256, id, m_l, m_m);
                }
                let base = id & !0xf00;
                if (OCEANIC..=FREEZING).contains(&base) {
                    gen_potential_inner(mc, flags, L::DeepOcean256, id, m_l, m_m);
                }
            } else {
                // (L_MUSHROOM_256, L_BIOME_256] for 1.6
                if id == OCEAN || id == MUSHROOM_FIELDS {
                    gen_potential_inner(mc, flags, L::Biome256, id, m_l, m_m);
                } else {
                    gen_potential_inner(mc, flags, L::Biome256, DESERT, m_l, m_m);
                    gen_potential_inner(mc, flags, L::Biome256, FOREST, m_l, m_m);
                    gen_potential_inner(mc, flags, L::Biome256, MOUNTAINS, m_l, m_m);
                    gen_potential_inner(mc, flags, L::Biome256, SWAMP, m_l, m_m);
                    gen_potential_inner(mc, flags, L::Biome256, PLAINS, m_l, m_m);
                    gen_potential_inner(mc, flags, L::Biome256, TAIGA, m_l, m_m);
                    if mc.is_at_least(MCVersion::V1_2) {
                        gen_potential_inner(mc, flags, L::Biome256, JUNGLE, m_l, m_m);
                    }
                    if id != PLAINS {
                        gen_potential_inner(mc, flags, L::Biome256, SNOWY_TUNDRA, m_l, m_m);
                    }
                }
            }
        }
        L::DeepOcean256 => {
            if !mc.is_at_least(MCVersion::V1_7) {
                return;
            }
            let base = id & !0xf00;
            let mutated = id & 0xf00;
            match base {
                1 /* Warm */ => {
                    if mutated != 0 {
                        gen_potential_inner(mc, flags, L::Biome256, BADLANDS_PLATEAU, m_l, m_m);
                        gen_potential_inner(mc, flags, L::Biome256, WOODED_BADLANDS_PLATEAU, m_l, m_m);
                    } else {
                        gen_potential_inner(mc, flags, L::Biome256, DESERT, m_l, m_m);
                        gen_potential_inner(mc, flags, L::Biome256, SAVANNA, m_l, m_m);
                        gen_potential_inner(mc, flags, L::Biome256, PLAINS, m_l, m_m);
                    }
                }
                2 /* Lush */ => {
                    if mutated != 0 {
                        gen_potential_inner(mc, flags, L::Biome256, JUNGLE, m_l, m_m);
                    } else {
                        gen_potential_inner(mc, flags, L::Biome256, FOREST, m_l, m_m);
                        gen_potential_inner(mc, flags, L::Biome256, DARK_FOREST, m_l, m_m);
                        gen_potential_inner(mc, flags, L::Biome256, MOUNTAINS, m_l, m_m);
                        gen_potential_inner(mc, flags, L::Biome256, PLAINS, m_l, m_m);
                        gen_potential_inner(mc, flags, L::Biome256, BIRCH_FOREST, m_l, m_m);
                        gen_potential_inner(mc, flags, L::Biome256, SWAMP, m_l, m_m);
                    }
                }
                3 /* Cold */ => {
                    if mutated != 0 {
                        gen_potential_inner(mc, flags, L::Biome256, GIANT_TREE_TAIGA, m_l, m_m);
                    } else {
                        gen_potential_inner(mc, flags, L::Biome256, FOREST, m_l, m_m);
                        gen_potential_inner(mc, flags, L::Biome256, MOUNTAINS, m_l, m_m);
                        gen_potential_inner(mc, flags, L::Biome256, TAIGA, m_l, m_m);
                        gen_potential_inner(mc, flags, L::Biome256, PLAINS, m_l, m_m);
                    }
                }
                FREEZING => {
                    gen_potential_inner(mc, flags, L::Biome256, SNOWY_TUNDRA, m_l, m_m);
                    gen_potential_inner(mc, flags, L::Biome256, SNOWY_TAIGA, m_l, m_m);
                }
                _ => {
                    gen_potential_inner(mc, flags, L::Biome256, base, m_l, m_m);
                }
            }
        }
        L::Biome256 | L::Bamboo256 | L::Zoom64 => {
            if !mc.is_at_least(MCVersion::V1_14) && layer == L::Bamboo256 {
                return;
            }
            if mc.is_at_least(MCVersion::V1_7) {
                if mc.is_at_least(MCVersion::V1_14) && id == JUNGLE {
                    gen_potential_inner(mc, flags, L::BiomeEdge64, BAMBOO_JUNGLE, m_l, m_m);
                }
                if id == WOODED_BADLANDS_PLATEAU || id == BADLANDS_PLATEAU {
                    gen_potential_inner(mc, flags, L::BiomeEdge64, BADLANDS, m_l, m_m);
                } else if id == GIANT_TREE_TAIGA {
                    gen_potential_inner(mc, flags, L::BiomeEdge64, TAIGA, m_l, m_m);
                } else if id == DESERT {
                    gen_potential_inner(mc, flags, L::BiomeEdge64, WOODED_MOUNTAINS, m_l, m_m);
                } else if id == SWAMP {
                    gen_potential_inner(mc, flags, L::BiomeEdge64, JUNGLE_EDGE, m_l, m_m);
                    gen_potential_inner(mc, flags, L::BiomeEdge64, PLAINS, m_l, m_m);
                }
                gen_potential_inner(mc, flags, L::BiomeEdge64, id, m_l, m_m);
            } else {
                // (L_BIOME_256, L_HILLS_64] for 1.6 — fallthrough to BiomeEdge64.
                handle_biome_edge_64(mc, flags, id, m_l, m_m);
            }
        }
        L::BiomeEdge64 => {
            if !mc.is_at_least(MCVersion::V1_7) {
                return;
            }
            handle_biome_edge_64(mc, flags, id, m_l, m_m);
        }
        L::Hills64 => {
            if mc.is_at_least(MCVersion::V1_7) {
                if id == PLAINS {
                    gen_potential_inner(mc, flags, L::Sunflower64, SUNFLOWER_PLAINS, m_l, m_m);
                }
                gen_potential_inner(mc, flags, L::Sunflower64, id, m_l, m_m);
            } else {
                // (L_HILLS_64, L_SHORE_16] for 1.6
                if id == MUSHROOM_FIELDS {
                    gen_potential_inner(mc, flags, L::Shore16, MUSHROOM_FIELD_SHORE, m_l, m_m);
                } else if id == MOUNTAINS {
                    gen_potential_inner(mc, flags, L::Shore16, MOUNTAIN_EDGE, m_l, m_m);
                } else if id != OCEAN && id != RIVER && id != SWAMP {
                    gen_potential_inner(mc, flags, L::Shore16, BEACH, m_l, m_m);
                }
                gen_potential_inner(mc, flags, L::Shore16, id, m_l, m_m);
            }
        }
        L::Sunflower64 => {
            if !mc.is_at_least(MCVersion::V1_7) {
                return;
            }
            handle_sunflower_or_zoom16(mc, flags, id, m_l, m_m);
        }
        L::Zoom16 => {
            if !mc.is_at_least(MCVersion::V1_1) && layer == L::Zoom16 {
                gen_potential_inner(mc, flags, L::Shore16, id, m_l, m_m);
                return;
            }
            handle_sunflower_or_zoom16(mc, flags, id, m_l, m_m);
        }
        L::Shore16 | L::SwampRiver16 | L::Zoom4 => {
            if id == SNOWY_TUNDRA {
                gen_potential_inner(mc, flags, L::RiverMix4, FROZEN_RIVER, m_l, m_m);
            } else if id == MUSHROOM_FIELDS || id == MUSHROOM_FIELD_SHORE {
                gen_potential_inner(mc, flags, L::RiverMix4, MUSHROOM_FIELD_SHORE, m_l, m_m);
            } else if id != OCEAN && (!mc.is_at_least(MCVersion::V1_7) || !Biome::is_oceanic_id(id))
            {
                gen_potential_inner(mc, flags, L::RiverMix4, RIVER, m_l, m_m);
            }
            gen_potential_inner(mc, flags, L::RiverMix4, id, m_l, m_m);
        }
        L::RiverMix4 => {
            if mc.is_at_least(MCVersion::V1_13) && Biome::is_oceanic_id(id) {
                if id == OCEAN {
                    gen_potential_inner(mc, flags, L::Voronoi1, OCEAN, m_l, m_m);
                    gen_potential_inner(mc, flags, L::Voronoi1, WARM_OCEAN, m_l, m_m);
                    gen_potential_inner(mc, flags, L::Voronoi1, LUKEWARM_OCEAN, m_l, m_m);
                    gen_potential_inner(mc, flags, L::Voronoi1, COLD_OCEAN, m_l, m_m);
                    gen_potential_inner(mc, flags, L::Voronoi1, FROZEN_OCEAN, m_l, m_m);
                } else if id == DEEP_OCEAN {
                    gen_potential_inner(mc, flags, L::Voronoi1, DEEP_OCEAN, m_l, m_m);
                    gen_potential_inner(mc, flags, L::Voronoi1, DEEP_LUKEWARM_OCEAN, m_l, m_m);
                    gen_potential_inner(mc, flags, L::Voronoi1, DEEP_COLD_OCEAN, m_l, m_m);
                    gen_potential_inner(mc, flags, L::Voronoi1, DEEP_FROZEN_OCEAN, m_l, m_m);
                } else {
                    return;
                }
            }
            gen_potential_inner(mc, flags, L::Voronoi1, id, m_l, m_m);
        }
        L::OceanMix4 => {
            if !mc.is_at_least(MCVersion::V1_13) {
                return;
            }
            gen_potential_inner(mc, flags, L::Voronoi1, id, m_l, m_m);
        }
        L::Voronoi1 => {
            or_bit(m_l, m_m, id);
        }
        _ => {
            // genPotential() not implemented for layer — silently ignore.
        }
    }
}

fn handle_biome_edge_64(mc: MCVersion, flags: u32, id: i32, m_l: &mut u64, m_m: &mut u64) {
    if !Biome::is_shallow_ocean_id(id) && Biome::get_mutated_id(mc, id) > 0 {
        gen_potential_inner(
            mc,
            flags,
            LayerId::Hills64,
            Biome::get_mutated_id(mc, id),
            m_l,
            m_m,
        );
    }
    match id {
        DESERT => {
            gen_potential_inner(mc, flags, LayerId::Hills64, DESERT_HILLS, m_l, m_m);
        }
        FOREST => {
            gen_potential_inner(mc, flags, LayerId::Hills64, WOODED_HILLS, m_l, m_m);
        }
        BIRCH_FOREST => {
            gen_potential_inner(mc, flags, LayerId::Hills64, BIRCH_FOREST_HILLS, m_l, m_m);
            gen_potential_inner(
                mc,
                flags,
                LayerId::Hills64,
                Biome::get_mutated_id(mc, BIRCH_FOREST_HILLS),
                m_l,
                m_m,
            );
        }
        DARK_FOREST => {
            gen_potential_inner(mc, flags, LayerId::Hills64, PLAINS, m_l, m_m);
            gen_potential_inner(
                mc,
                flags,
                LayerId::Hills64,
                Biome::get_mutated_id(mc, PLAINS),
                m_l,
                m_m,
            );
        }
        TAIGA => {
            gen_potential_inner(mc, flags, LayerId::Hills64, TAIGA_HILLS, m_l, m_m);
        }
        GIANT_TREE_TAIGA => {
            gen_potential_inner(
                mc,
                flags,
                LayerId::Hills64,
                GIANT_TREE_TAIGA_HILLS,
                m_l,
                m_m,
            );
            gen_potential_inner(
                mc,
                flags,
                LayerId::Hills64,
                Biome::get_mutated_id(mc, GIANT_TREE_TAIGA_HILLS),
                m_l,
                m_m,
            );
        }
        SNOWY_TAIGA => {
            gen_potential_inner(mc, flags, LayerId::Hills64, SNOWY_TAIGA_HILLS, m_l, m_m);
        }
        PLAINS => {
            if mc.is_at_least(MCVersion::V1_7) {
                gen_potential_inner(mc, flags, LayerId::Hills64, WOODED_HILLS, m_l, m_m);
            }
            gen_potential_inner(mc, flags, LayerId::Hills64, FOREST, m_l, m_m);
            gen_potential_inner(
                mc,
                flags,
                LayerId::Hills64,
                Biome::get_mutated_id(mc, FOREST),
                m_l,
                m_m,
            );
        }
        SNOWY_TUNDRA => {
            gen_potential_inner(mc, flags, LayerId::Hills64, SNOWY_MOUNTAINS, m_l, m_m);
        }
        JUNGLE => {
            gen_potential_inner(mc, flags, LayerId::Hills64, JUNGLE_HILLS, m_l, m_m);
        }
        BAMBOO_JUNGLE => {
            gen_potential_inner(mc, flags, LayerId::Hills64, BAMBOO_JUNGLE_HILLS, m_l, m_m);
        }
        OCEAN => {
            if mc.is_at_least(MCVersion::V1_7) {
                gen_potential_inner(mc, flags, LayerId::Hills64, DEEP_OCEAN, m_l, m_m);
            }
        }
        MOUNTAINS => {
            if mc.is_at_least(MCVersion::V1_7) {
                gen_potential_inner(mc, flags, LayerId::Hills64, WOODED_MOUNTAINS, m_l, m_m);
                gen_potential_inner(
                    mc,
                    flags,
                    LayerId::Hills64,
                    Biome::get_mutated_id(mc, WOODED_MOUNTAINS),
                    m_l,
                    m_m,
                );
            }
        }
        SAVANNA => {
            gen_potential_inner(mc, flags, LayerId::Hills64, SAVANNA_PLATEAU, m_l, m_m);
            gen_potential_inner(
                mc,
                flags,
                LayerId::Hills64,
                Biome::get_mutated_id(mc, SAVANNA_PLATEAU),
                m_l,
                m_m,
            );
        }
        _ => {
            if Biome::are_similar_ids(mc, id, WOODED_BADLANDS_PLATEAU) {
                gen_potential_inner(mc, flags, LayerId::Hills64, BADLANDS, m_l, m_m);
                gen_potential_inner(
                    mc,
                    flags,
                    LayerId::Hills64,
                    Biome::get_mutated_id(mc, BADLANDS),
                    m_l,
                    m_m,
                );
            } else if Biome::is_deep_ocean_id(id) {
                gen_potential_inner(mc, flags, LayerId::Hills64, PLAINS, m_l, m_m);
                gen_potential_inner(mc, flags, LayerId::Hills64, FOREST, m_l, m_m);
                gen_potential_inner(
                    mc,
                    flags,
                    LayerId::Hills64,
                    Biome::get_mutated_id(mc, PLAINS),
                    m_l,
                    m_m,
                );
                gen_potential_inner(
                    mc,
                    flags,
                    LayerId::Hills64,
                    Biome::get_mutated_id(mc, FOREST),
                    m_l,
                    m_m,
                );
            }
        }
    }
    gen_potential_inner(mc, flags, LayerId::Hills64, id, m_l, m_m);
}

fn handle_sunflower_or_zoom16(mc: MCVersion, flags: u32, id: i32, m_l: &mut u64, m_m: &mut u64) {
    if id == MUSHROOM_FIELDS {
        gen_potential_inner(mc, flags, LayerId::Shore16, MUSHROOM_FIELD_SHORE, m_l, m_m);
    } else if Biome::get_category_id(mc, id) == JUNGLE {
        gen_potential_inner(mc, flags, LayerId::Shore16, BEACH, m_l, m_m);
        gen_potential_inner(mc, flags, LayerId::Shore16, JUNGLE_EDGE, m_l, m_m);
    } else if id == MOUNTAINS || id == WOODED_MOUNTAINS || id == MOUNTAIN_EDGE {
        gen_potential_inner(mc, flags, LayerId::Shore16, STONE_SHORE, m_l, m_m);
    } else if Biome::is_snowy_id(id) {
        gen_potential_inner(mc, flags, LayerId::Shore16, SNOWY_BEACH, m_l, m_m);
    } else if id == BADLANDS || id == WOODED_BADLANDS_PLATEAU {
        gen_potential_inner(mc, flags, LayerId::Shore16, DESERT, m_l, m_m);
    } else if id != OCEAN && id != DEEP_OCEAN && id != RIVER && id != SWAMP {
        gen_potential_inner(mc, flags, LayerId::Shore16, BEACH, m_l, m_m);
    }
    gen_potential_inner(mc, flags, LayerId::Shore16, id, m_l, m_m);
}
