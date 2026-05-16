//! Structure-position viability predicates.
//!
//! Bit-exact port of cubiomes' `isViableFeatureBiome` and the
//! supporting `isViableStructurePos` switch from `finders.c`. This
//! commit ships the per-biome eligibility check; the full
//! `is_viable_structure_pos` (which samples biomes around the
//! candidate point) lands in a follow-up.

use crate::biome::Biome;
use crate::finder::{StructureType, get_structure_config, get_structure_pos};
use crate::generator::Generator;
use crate::math::floordiv;
use crate::mc_version::{Dimension, MCVersion};

/// `isViableFeatureBiome(mc, structureType, biomeID)` — return
/// `true` when `biome_id` is one of the biomes the given structure
/// type is allowed to generate in for the given MC version.
///
/// Cubiomes panics (`fprintf(stderr); exit(1)`) for unimplemented
/// types; we mirror that by panicking on the catch-all arm so the
/// caller sees the same fatal-error semantics.
#[must_use]
#[allow(clippy::too_many_lines, clippy::match_same_arms)]
pub fn is_viable_feature_biome(
    mc: MCVersion,
    structure_type: StructureType,
    biome_id: i32,
) -> bool {
    use StructureType::*;
    match structure_type {
        DesertPyramid => biome_id == Biome::DESERT.id() || biome_id == 17, // desert_hills
        JungleTemple => matches!(biome_id, 21 | 22 | 168 | 169), // jungle / jungle_hills / bamboo_jungle / bamboo_jungle_hills
        SwampHut => biome_id == Biome::SWAMP.id(),
        Igloo => {
            if !mc.is_at_least(MCVersion::V1_9) {
                return false;
            }
            // snowy_tundra (= snowy_plains), snowy_taiga, snowy_slopes
            matches!(biome_id, 12 | 30 | 179)
        }
        OceanRuin => {
            if !mc.is_at_least(MCVersion::V1_13) {
                return false;
            }
            Biome::is_oceanic_id(biome_id)
        }
        Shipwreck => {
            if !mc.is_at_least(MCVersion::V1_13) {
                return false;
            }
            Biome::is_oceanic_id(biome_id) || biome_id == 16 /* beach */ || biome_id == 26 /* snowy_beach */
        }
        RuinedPortal | RuinedPortalN => mc.is_at_least(MCVersion::V1_16_1),
        AncientCity => {
            if !mc.is_at_least(MCVersion::V1_19_2) {
                return false;
            }
            biome_id == Biome::DEEP_DARK.id()
        }
        TrailRuins => {
            if !mc.is_at_least(MCVersion::V1_20) {
                return false;
            }
            // taiga, snowy_taiga, old_growth_pine_taiga (=giant_tree_taiga),
            // old_growth_spruce_taiga (=giant_spruce_taiga),
            // old_growth_birch_forest (=tall_birch_forest), jungle
            matches!(biome_id, 5 | 30 | 32 | 160 | 155 | 21)
        }
        TrialChambers => {
            if !mc.is_at_least(MCVersion::V1_21_1) {
                return false;
            }
            biome_id != Biome::DEEP_DARK.id() && Biome::is_overworld_id(mc, biome_id)
        }
        Treasure => {
            if !mc.is_at_least(MCVersion::V1_13) {
                return false;
            }
            biome_id == 16 /* beach */ || biome_id == 26 /* snowy_beach */
        }
        Mineshaft => Biome::is_overworld_id(mc, biome_id),
        DesertWell => biome_id == Biome::DESERT.id(),
        Monument => {
            if !mc.is_at_least(MCVersion::V1_8) {
                return false;
            }
            Biome::is_deep_ocean_id(biome_id)
        }
        Outpost => {
            if !mc.is_at_least(MCVersion::V1_14) {
                return false;
            }
            if mc.is_at_least(MCVersion::V1_18) {
                // desert, plains, savanna, snowy_plains (=snowy_tundra), taiga,
                // meadow, frozen_peaks, jagged_peaks, stony_peaks,
                // snowy_slopes, grove, cherry_grove
                matches!(
                    biome_id,
                    2 | 1 | 35 | 12 | 5 | 177 | 181 | 180 | 182 | 179 | 178 | 185
                )
            } else {
                village_viable(mc, biome_id)
            }
        }
        Village => village_viable(mc, biome_id),
        Mansion => {
            if !mc.is_at_least(MCVersion::V1_11) {
                return false;
            }
            biome_id == Biome::DARK_FOREST.id() || biome_id == 157 // dark_forest_hills
        }
        Fortress => matches!(biome_id, 8 | 170 | 172 | 171 | 173),
        // nether_wastes / soul_sand_valley / warped_forest /
        // crimson_forest / basalt_deltas
        Bastion => {
            if !mc.is_at_least(MCVersion::V1_16_1) {
                return false;
            }
            // Same as Fortress minus basalt_deltas.
            matches!(biome_id, 8 | 170 | 172 | 171)
        }
        EndCity => {
            if !mc.is_at_least(MCVersion::V1_9) {
                return false;
            }
            biome_id == Biome::END_MIDLANDS.id() || biome_id == Biome::END_HIGHLANDS.id()
        }
        EndGateway => {
            if !mc.is_at_least(MCVersion::V1_13) {
                return false;
            }
            biome_id == Biome::END_HIGHLANDS.id()
        }
        // Not implemented in cubiomes itself — Feature, EndIsland,
        // Geode never call isViableFeatureBiome (they have
        // type-specific logic in isViableStructurePos).
        Feature | EndIsland | Geode => {
            panic!("is_viable_feature_biome: not implemented for {structure_type:?}");
        }
    }
}

/// `isViableStructurePos(structureType, g, x, z, flags)` — return
/// `true` when the block-coordinate `(x, z)` position is allowed
/// to host the given structure given the world state in `g`.
///
/// **Partial port**: this commit ships the Nether and End branches
/// of cubiomes' `isViableStructurePos`. The Overworld branch
/// requires the `mapViableBiome` / `mapViableShore` layer-hook
/// machinery (cubiomes monkey-patches `getMap` on two layers and
/// relies on early-return out of layer evaluation), plus
/// `getVariant` for 1.18+ Bastion / Village placement. Those land
/// in a follow-up stage; calling this function with an Overworld
/// `Generator` panics.
///
/// Returns `true`/`false`. Cubiomes returns the biome id for some
/// Village arms ("for further analysis") but we model that as a
/// boolean; callers needing the biome can sample directly.
#[must_use]
pub fn is_viable_structure_pos(
    structure_type: StructureType,
    g: &Generator,
    x: i32,
    z: i32,
    _flags: u32,
) -> bool {
    let dim = g
        .dim
        .expect("is_viable_structure_pos: generator must have apply_seed'd dim");
    let chunk_x = (x >> 4) as i64;
    let chunk_z = (z >> 4) as i64;

    if dim == Dimension::Nether {
        return viable_nether(structure_type, g, x, z, chunk_x, chunk_z);
    }
    if dim == Dimension::End {
        return viable_end(structure_type, g, chunk_x, chunk_z);
    }
    // Overworld: requires the mapViableBiome layer hook + getVariant.
    unimplemented!(
        "Overworld is_viable_structure_pos requires the mapViableBiome layer hook (follow-up stage)"
    );
}

fn viable_nether(
    structure_type: StructureType,
    g: &Generator,
    x: i32,
    z: i32,
    chunk_x: i64,
    chunk_z: i64,
) -> bool {
    use StructureType::*;
    if structure_type == Fortress && g.mc.is_before(MCVersion::V1_18) {
        return true;
    }
    if g.mc.is_before(MCVersion::V1_16_1) {
        return false;
    }
    if structure_type == RuinedPortalN {
        return true;
    }
    if structure_type == Fortress {
        // 1.18+: fortresses generate wherever bastions don't.
        let Some(sc) = get_structure_config(Fortress, g.mc) else {
            return false;
        };
        let region_blocks = i32::from(sc.region_size) << 4;
        let rp_x = floordiv(x, region_blocks);
        let rp_z = floordiv(z, region_blocks);
        if get_structure_pos(Bastion, g.mc, g.seed, rp_x, rp_z).is_none() {
            return true;
        }
        return !is_viable_structure_pos(Bastion, g, x, z, 0);
    }
    // Bastion 1.18+ needs getVariant; defer.
    if g.mc.is_at_least(MCVersion::V1_18) && structure_type == Bastion {
        unimplemented!("Bastion 1.18+ viability needs getVariant (follow-up)");
    }
    let sample_x = (chunk_x as i32) * 4 + 2;
    let sample_z = (chunk_z as i32) * 4 + 2;
    let id = g.biome_at(4, sample_x, 0, sample_z).0;
    is_viable_feature_biome(g.mc, structure_type, id)
}

fn viable_end(structure_type: StructureType, g: &Generator, chunk_x: i64, chunk_z: i64) -> bool {
    use StructureType::*;
    match structure_type {
        EndCity => {
            if g.mc.is_before(MCVersion::V1_9) {
                return false;
            }
        }
        EndGateway => {
            if g.mc.is_before(MCVersion::V1_13) {
                return false;
            }
        }
        _ => return false,
    }
    // End biomes vary only on a per-chunk scale (1:16).
    let id = g.biome_at(16, chunk_x as i32, 0, chunk_z as i32).0;
    is_viable_feature_biome(g.mc, structure_type, id)
}

/// Cubiomes' `Village` viability — also used by `Outpost` on
/// pre-1.18 MC versions (the latter falls through to this case).
fn village_viable(mc: MCVersion, biome_id: i32) -> bool {
    if matches!(
        biome_id,
        2 /* desert */ | 1 /* plains */ | 35 /* savanna */
    ) {
        return true;
    }
    if mc.is_at_least(MCVersion::V1_10) && biome_id == Biome::TAIGA.id() {
        return true;
    }
    if mc.is_at_least(MCVersion::V1_14) && biome_id == Biome::SNOWY_TUNDRA.id() {
        return true;
    }
    if mc.is_at_least(MCVersion::V1_18) && biome_id == Biome::MEADOW.id() {
        return true;
    }
    false
}
