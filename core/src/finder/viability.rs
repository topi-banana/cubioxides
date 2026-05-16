//! Structure-position viability predicates.
//!
//! Bit-exact port of cubiomes' `isViableFeatureBiome` and the
//! supporting `isViableStructurePos` switch from `finders.c`. This
//! commit ships the per-biome eligibility check; the full
//! `is_viable_structure_pos` (which samples biomes around the
//! candidate point) lands in a follow-up.

#![allow(clippy::collapsible_match, clippy::needless_return)]

use crate::biome::Biome;
use crate::finder::variant::get_variant;
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
    // Overworld: 1.18+ uses BiomeNoise sampling and does NOT need the
    // legacy `mapViableBiome` / `mapViableShore` layer hooks; pre-1.18
    // does need them for performance, but we can match cubiomes
    // bit-exactly by skipping the early-exit optimisation and just
    // doing the final biome check at the standard sample point.
    if g.mc.is_at_least(MCVersion::V1_18) {
        return viable_overworld_modern(structure_type, g, x, z, chunk_x, chunk_z);
    }
    unimplemented!(
        "Pre-1.18 Overworld is_viable_structure_pos requires more `getStructureConfig` + biome-check work (follow-up stage)"
    );
}

#[allow(clippy::too_many_lines)]
fn viable_overworld_modern(
    structure_type: StructureType,
    g: &Generator,
    x: i32,
    z: i32,
    chunk_x: i64,
    chunk_z: i64,
) -> bool {
    use StructureType::*;
    match structure_type {
        // Always-viable / unsupported gates first.
        Mineshaft => return true,
        RuinedPortal | RuinedPortalN => return g.mc.is_at_least(MCVersion::V1_16_1),
        Geode => return g.mc.is_at_least(MCVersion::V1_17),

        Village => return viable_village_modern(g, x, z, chunk_x, chunk_z),

        // L_feature path (1.18+: sample at chunkX*4+2, y=319>>2).
        DesertPyramid | JungleTemple | SwampHut | OceanRuin | Shipwreck | Treasure | Igloo
        | TrailRuins => {
            // Per-structure MC gates from cubiomes.
            match structure_type {
                TrailRuins => {
                    if g.mc.is_before(MCVersion::V1_20) {
                        return false;
                    }
                }
                OceanRuin | Shipwreck | Treasure => {
                    if g.mc.is_before(MCVersion::V1_13) {
                        return false;
                    }
                }
                Igloo => {
                    if g.mc.is_before(MCVersion::V1_9) {
                        return false;
                    }
                }
                _ => {}
            }
            let sample_x = (chunk_x * 4 + 2) as i32;
            let sample_z = (chunk_z * 4 + 2) as i32;
            let id = modern_biome_at_scale0(g, sample_x, 319 >> 2, sample_z);
            if id < 0 {
                return false;
            }
            is_viable_feature_biome(g.mc, structure_type, id)
        }

        DesertWell => {
            let id = modern_biome_at_scale0(g, x >> 2, 319 >> 2, z >> 2);
            if id < 0 {
                return false;
            }
            is_viable_feature_biome(g.mc, structure_type, id)
        }

        Mansion => {
            // 1.18+: cubiomes' TODO note — biome check at center,
            // ignoring the surface-height minimum.
            let sample_x = (chunk_x * 16 + 7) as i32;
            let sample_z = (chunk_z * 16 + 7) as i32;
            let id = g.biome_at(4, sample_x >> 2, 319 >> 2, sample_z >> 2).0;
            if id < 0 {
                return false;
            }
            is_viable_feature_biome(g.mc, structure_type, id)
        }

        AncientCity | TrialChambers => {
            // L_jigsaw path (1.19_2+ / 1.21_1+).
            if structure_type == AncientCity && g.mc.is_before(MCVersion::V1_19_2) {
                return false;
            }
            if structure_type == TrialChambers && g.mc.is_before(MCVersion::V1_21_1) {
                return false;
            }
            let sv = get_variant(structure_type, g.mc, g.seed, x, z, -1)
                .expect("L_jigsaw getVariant must succeed");
            let sample_x =
                (((chunk_x * 32 + 2 * i64::from(sv.x) + i64::from(sv.sx) - 1) / 2) >> 2) as i32;
            let sample_z =
                (((chunk_z * 32 + 2 * i64::from(sv.z) + i64::from(sv.sz) - 1) / 2) >> 2) as i32;
            let sample_y = i32::from(sv.y) >> 2;
            let id = g.biome_at(4, sample_x, sample_y, sample_z).0;
            if id < 0 {
                return false;
            }
            is_viable_feature_biome(g.mc, structure_type, id)
        }

        // Bastion / Fortress are Nether-only — should never reach
        // here. Feature is pre-1.13 only. Defer Monument / Outpost /
        // EndCity / EndGateway / EndIsland to follow-up sub-stages.
        Feature | Monument | Outpost | EndCity | EndGateway | EndIsland | Fortress | Bastion => {
            unimplemented!(
                "Modern Overworld is_viable_structure_pos: {structure_type:?} not yet ported"
            )
        }
    }
}

/// Cubiomes' `getBiomeAt(g, 0, x, y, z)` for 1.18+ Overworld
/// resolves to `genBiomeNoise3D(..., scale=1, mid=0)` which is
/// equivalent to sampling `BiomeNoise` directly at `(x, y, z)`
/// (scale-4 cell coords with no voronoi). Rust's
/// `Generator::biome_at` rejects scale=0 explicitly, so we bypass
/// it here.
fn modern_biome_at_scale0(g: &Generator, x: i32, y: i32, z: i32) -> i32 {
    let bn = g
        .biome_noise
        .as_ref()
        .expect("modern Overworld must have BiomeNoise");
    bn.sample(x, y, z, 0).0
}

/// 1.18+ Village viability: per-biome `getVariant` loop with a
/// biome-check at the variant centroid. Mirrors cubiomes' inner
/// `vv[]` loop.
fn viable_village_modern(g: &Generator, x: i32, z: i32, chunk_x: i64, chunk_z: i64) -> bool {
    use crate::finder::variant::get_variant;
    // Village biomes cubiomes 1.18+ tries in order: plains, desert,
    // savanna, taiga, snowy_tundra.
    const VV: [i32; 5] = [1, 2, 35, 5, 12];
    const MEADOW: i32 = 177;
    for vi in VV {
        let Some(sv) = get_variant(StructureType::Village, g.mc, g.seed, x, z, vi) else {
            continue;
        };
        let sample_x =
            (((chunk_x * 32 + 2 * i64::from(sv.x) + i64::from(sv.sx) - 1) / 2) >> 2) as i32;
        let sample_z =
            (((chunk_z * 32 + 2 * i64::from(sv.z) + i64::from(sv.sz) - 1) / 2) >> 2) as i32;
        let sample_y = 319 >> 2;
        let id = modern_biome_at_scale0(g, sample_x, sample_y, sample_z);
        if id == vi || (id == MEADOW && vi == 1) {
            return true;
        }
    }
    false
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
    // Bastion 1.18+: sample at the variant centroid.
    let (sample_x, sample_z, sample_y);
    if g.mc.is_at_least(MCVersion::V1_18) && structure_type == Bastion {
        let sv = get_variant(Bastion, g.mc, g.seed, x, z, -1)
            .expect("Bastion 1.18+ getVariant must succeed");
        sample_x = (((chunk_x * 32 + 2 * sv.x as i64 + sv.sx as i64 - 1) / 2) >> 2) as i32;
        sample_z = (((chunk_z * 32 + 2 * sv.z as i64 + sv.sz as i64 - 1) / 2) >> 2) as i32;
        sample_y = if g.mc.is_at_least(MCVersion::V1_19_2) {
            33 >> 2
        } else {
            0
        };
    } else {
        sample_x = (chunk_x as i32) * 4 + 2;
        sample_z = (chunk_z as i32) * 4 + 2;
        sample_y = 0;
    }
    let id = g.biome_at(4, sample_x, sample_y, sample_z).0;
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
