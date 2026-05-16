//! Structure-position viability predicates.
//!
//! Bit-exact port of cubiomes' `isViableFeatureBiome` and the
//! supporting `isViableStructurePos` switch from `finders.c`. This
//! commit ships the per-biome eligibility check; the full
//! `is_viable_structure_pos` (which samples biomes around the
//! candidate point) lands in a follow-up.

#![allow(
    clippy::collapsible_match,
    clippy::needless_return,
    clippy::doc_markdown
)]

use crate::biome::Biome;
use crate::finder::population_seed::chunk_generate_rng;
use crate::finder::variant::get_variant;
use crate::finder::{
    StructureType, get_feature_pos, get_structure_config, get_structure_pos, set_attempt_seed,
};
use crate::generator::{Generator, Range};
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
    viable_overworld_legacy(structure_type, g, x, z, chunk_x, chunk_z)
}

/// Pre-1.18 Overworld viability. Cubiomes patches the
/// `mapViableBiome` / `mapViableShore` layer-hook for performance
/// (early-exit during biome generation when an invalid biome is
/// seen); we skip the hook and just do the final biome check at
/// the cubiomes-specific sample point.
#[allow(clippy::too_many_lines)]
fn viable_overworld_legacy(
    structure_type: StructureType,
    g: &Generator,
    x: i32,
    z: i32,
    chunk_x: i64,
    chunk_z: i64,
) -> bool {
    use StructureType::*;
    match structure_type {
        // Always-viable / version-gated trivial cases.
        Mineshaft => true,
        RuinedPortal | RuinedPortalN => g.mc.is_at_least(MCVersion::V1_16_1),
        Geode => g.mc.is_at_least(MCVersion::V1_17),
        AncientCity | TrialChambers => false, // 1.19_2+ / 1.21_1+ only.

        // L_feature path: Desert/Jungle/Swamp temples, Igloo,
        // Ocean_Ruin, Shipwreck, Treasure, Trail_Ruins.
        DesertPyramid | JungleTemple | SwampHut | Igloo | OceanRuin | Shipwreck | Treasure
        | TrailRuins => {
            // Per-structure MC gates.
            match structure_type {
                TrailRuins => return false, // 1.20+ only, but 1.20 is 1.18+.
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
            let (sample_x, sample_z, scale) = legacy_feature_sample(g.mc, chunk_x, chunk_z);
            let id = g.biome_at(scale, sample_x, 319 >> 2, sample_z).0;
            if id < 0 {
                return false;
            }
            is_viable_feature_biome(g.mc, structure_type, id)
        }

        DesertWell => {
            let (sample_x, sample_z, scale) = if g.mc.is_before(MCVersion::V1_16_1) {
                (x, z, 1)
            } else {
                (x >> 2, z >> 2, 4)
            };
            let id = g.biome_at(scale, sample_x, 319 >> 2, sample_z).0;
            if id < 0 {
                return false;
            }
            is_viable_feature_biome(g.mc, structure_type, id)
        }

        Village => viable_village_legacy(g, chunk_x, chunk_z),

        Monument => viable_monument_legacy(g, chunk_x, chunk_z),

        Mansion => viable_mansion_legacy(g, chunk_x, chunk_z),

        Outpost => viable_outpost_legacy(g, x, z, chunk_x, chunk_z),

        Feature | EndCity | EndGateway | EndIsland | Fortress | Bastion => {
            // Wrong-dim or shouldn't reach here.
            unimplemented!("legacy Overworld is_viable_structure_pos: {structure_type:?}")
        }
    }
}

/// Cubiomes' pre-1.18 L_feature sample point + entry-layer
/// selection. Returns `(sample_x, sample_z, scale)`.
fn legacy_feature_sample(mc: MCVersion, chunk_x: i64, chunk_z: i64) -> (i32, i32, i32) {
    if mc.is_before(MCVersion::V1_16_1) {
        // Pre-1.16: sample at chunk center (block coords) via L_VORONOI_1.
        ((chunk_x * 16 + 9) as i32, (chunk_z * 16 + 9) as i32, 1)
    } else {
        // 1.16-1.17: sample at chunk*4+2 via L_RIVER_MIX_4.
        ((chunk_x * 4 + 2) as i32, (chunk_z * 4 + 2) as i32, 4)
    }
}

/// Pre-1.18 Outpost viability. Cubiomes runs three gates:
///   1. `setAttemptSeed` + `nextInt(5) == 0` per-chunk roll.
///   2. No Village placed within ±10 chunks. For MC < 1.16.1 the
///      proximity check additionally requires the would-be Village
///      position to pass `isViableStructurePos(Village, …)` — for
///      1.16.1+ any region-grid hit short-circuits without the
///      recursive check.
///   3. Biome at the chunk-centre sample point matches the Outpost
///      biome mask. Pre-1.16 samples via L_VORONOI_1 (`scale=1` at
///      `chunkX*16+9`); 1.16.1-1.17 samples via L_RIVER_MIX_4
///      (`scale=4` at `chunkX*4+2`).
fn viable_outpost_legacy(g: &Generator, x: i32, z: i32, chunk_x: i64, chunk_z: i64) -> bool {
    if g.mc.is_before(MCVersion::V1_14) {
        return false;
    }
    let mut rng = set_attempt_seed(g.seed, x >> 4, z >> 4);
    if rng.next_int(5) != 0 {
        return false;
    }
    let Some(vilconf) = get_structure_config(StructureType::Village, g.mc) else {
        return false;
    };
    let cx0 = chunk_x - 10;
    let cx1 = chunk_x + 10;
    let cz0 = chunk_z - 10;
    let cz1 = chunk_z + 10;
    let region = i32::from(vilconf.region_size);
    let rx0 = floordiv(cx0 as i32, region);
    let rx1 = floordiv(cx1 as i32, region);
    let rz0 = floordiv(cz0 as i32, region);
    let rz1 = floordiv(cz1 as i32, region);
    let is_pre_1_16_1 = g.mc.is_before(MCVersion::V1_16_1);
    for rz in rz0..=rz1 {
        for rx in rx0..=rx1 {
            let p = crate::finder::get_feature_pos(vilconf, g.seed, rx, rz);
            let pc_x = (p.x >> 4) as i64;
            let pc_z = (p.z >> 4) as i64;
            if pc_x >= cx0 && pc_x <= cx1 && pc_z >= cz0 && pc_z <= cz1 {
                if is_pre_1_16_1 {
                    // Recursive Village viability check — only
                    // viable villages disqualify the Outpost.
                    if is_viable_structure_pos(StructureType::Village, g, p.x, p.z, 0) {
                        return false;
                    }
                } else {
                    // 1.16.1+: short-circuit on any region-grid hit.
                    return false;
                }
            }
        }
    }
    // Biome sample at the chunk-centre point with per-MC sample
    // pattern (matches the L_feature path).
    let (sample_x, sample_z, scale) = if g.mc.is_before(MCVersion::V1_16_1) {
        ((chunk_x * 16 + 9) as i32, (chunk_z * 16 + 9) as i32, 1)
    } else {
        ((chunk_x * 4 + 2) as i32, (chunk_z * 4 + 2) as i32, 4)
    };
    let id = g.biome_at(scale, sample_x, 319 >> 2, sample_z).0;
    if id < 0 {
        return false;
    }
    is_viable_feature_biome(g.mc, StructureType::Outpost, id)
}

/// Pre-1.18 Monument viability. Three sub-paths:
///   - MC ≤ 1.7: never viable.
///   - MC == 1.8: single scale-1 sample (chunkX*16+8, _, chunkZ*16+8)
///     must be deep-ocean.
///   - MC 1.9–1.17: pre-check at scale-16 (Shore16 entry) must be
///     deep-ocean, then `areBiomesViable` over a 16-radius square
///     with `g_monument_biomes2` (deep oceans only), then another
///     check over 29 radius with `g_monument_biomes1` (any ocean
///     or river).
fn viable_monument_legacy(g: &Generator, chunk_x: i64, chunk_z: i64) -> bool {
    if g.mc.is_before(MCVersion::V1_8) {
        return false;
    }
    let sample_x = (chunk_x * 16 + 8) as i32;
    let sample_z = (chunk_z * 16 + 8) as i32;
    if g.mc == MCVersion::V1_8 {
        // 1.8 path: single-sample pre-check at the chunk centre
        // (block scale=1) — must be deep-ocean before the final
        // radius-29 check runs.
        let id = g.biome_at(1, sample_x, 0, sample_z).0;
        if id < 0 || !Biome::is_deep_ocean_id(id) {
            return false;
        }
    } else {
        // 1.9 - 1.17. Pre-check at scale-16 (Shore16 entry), then
        // a 16-radius areBiomesViable over deep-ocean variants only.
        let id = g.biome_at(16, chunk_x as i32, 0, chunk_z as i32).0;
        if id < 0 || !Biome::is_deep_ocean_id(id) {
            return false;
        }
        if !are_biomes_viable_legacy(g, sample_x, 63, sample_z, 16, G_MONUMENT_BIOMES2, 0) {
            return false;
        }
    }
    // Always run the final 29-radius any-ocean / river check.
    are_biomes_viable_legacy(g, sample_x, 63, sample_z, 29, G_MONUMENT_BIOMES1, 0)
}

/// Cubiomes' `g_monument_biomes2` — deep-ocean-only mask for the
/// 16-radius Monument viability pre-check.
const G_MONUMENT_BIOMES2: u64 = (1u64 << Biome::DEEP_FROZEN_OCEAN.id())
    | (1u64 << Biome::DEEP_COLD_OCEAN.id())
    | (1u64 << Biome::DEEP_OCEAN.id())
    | (1u64 << Biome::DEEP_LUKEWARM_OCEAN.id())
    | (1u64 << Biome::DEEP_WARM_OCEAN.id());

/// Pre-1.18 Mansion viability. Requires `dark_forest` (id 29) or
/// `dark_forest_hills` (id 157, > 128 so encoded in mask `m`)
/// across the full 32-radius square around (chunkX*16+8, _,
/// chunkZ*16+8).
fn viable_mansion_legacy(g: &Generator, chunk_x: i64, chunk_z: i64) -> bool {
    if g.mc.is_before(MCVersion::V1_11) {
        return false;
    }
    let sample_x = (chunk_x * 16 + 8) as i32;
    let sample_z = (chunk_z * 16 + 8) as i32;
    let b = 1u64 << Biome::DARK_FOREST.id();
    let m = 1u64 << (157 - 128); // dark_forest_hills (157)
    are_biomes_viable_legacy(g, sample_x, 0, sample_z, 32, b, m)
}

/// Pre-1.18 subset of cubiomes' `areBiomesViable`. Generates a
/// scale-4 biome cache over a `±rad/4` cell window centred on the
/// `(x, y, z) >> 2` sample point and returns `true` iff every cell
/// matches the `(valid_b, valid_m)` biome mask.
fn are_biomes_viable_legacy(
    g: &Generator,
    x: i32,
    y: i32,
    z: i32,
    rad: i32,
    valid_b: u64,
    valid_m: u64,
) -> bool {
    let x1 = (x - rad) >> 2;
    let x2 = (x + rad) >> 2;
    let sx = (x2 - x1 + 1) as u32;
    let z1 = (z - rad) >> 2;
    let z2 = (z + rad) >> 2;
    let sz = (z2 - z1 + 1) as u32;
    let y4 = (y - rad) >> 2;

    // Check corners first (matches cubiomes — also a perf win since
    // a corner mismatch saves the full cache allocation).
    let corners = [(x1, z1), (x2, z2), (x1, z2), (x2, z1)];
    for (cx, cz) in corners {
        let id = g.biome_at(4, cx, y4, cz).0;
        if id < 0 || !crate::finder::locate_biome::id_matches(id, valid_b, valid_m) {
            return false;
        }
    }

    // Full grid via the layered biome cache.
    let r = Range {
        scale: 4,
        x: x1,
        z: z1,
        sx,
        sz,
        y: y4,
        sy: 1,
    };
    let mut cache = vec![Biome::default(); r.cell_count()];
    g.gen_biomes(&mut cache, r);
    for c in cache.iter().take(r.cell_count()) {
        let id = c.0;
        if id < 0 || !crate::finder::locate_biome::id_matches(id, valid_b, valid_m) {
            return false;
        }
    }
    true
}

/// Pre-1.18 Village viability. MC_1_15 exclusively uses the
/// L_VORONOI_1 sample like other features; everything else uses
/// L_RIVER_MIX_4 at chunk*4+2. Pre-1.10 has an extra `getBiomeAt`
/// check at (chunkX*16+2, _, chunkZ*16+2) with scale=1 to verify
/// the corner of the chunk is also viable.
fn viable_village_legacy(g: &Generator, chunk_x: i64, chunk_z: i64) -> bool {
    let (sample_x, sample_z, scale) = if g.mc == MCVersion::V1_15 {
        ((chunk_x * 16 + 9) as i32, (chunk_z * 16 + 9) as i32, 1)
    } else {
        ((chunk_x * 4 + 2) as i32, (chunk_z * 4 + 2) as i32, 4)
    };
    let id = g.biome_at(scale, sample_x, 0, sample_z).0;
    if id < 0 || !is_viable_feature_biome(g.mc, StructureType::Village, id) {
        return false;
    }
    if g.mc.is_before(MCVersion::V1_10) {
        // Pre-1.10: also check the chunk-corner (2, 2) at scale=1.
        let sx2 = (chunk_x * 16 + 2) as i32;
        let sz2 = (chunk_z * 16 + 2) as i32;
        let id2 = g.biome_at(1, sx2, 0, sz2).0;
        if id2 < 0 || !is_viable_feature_biome(g.mc, StructureType::Village, id2) {
            return false;
        }
    }
    true
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

        Outpost => return viable_outpost_modern(g, x, z, chunk_x, chunk_z),

        Monument => return viable_monument_modern(g, chunk_x, chunk_z),

        // Bastion / Fortress are Nether-only — should never reach
        // here. Feature is pre-1.13 only. Defer EndCity / EndGateway
        // / EndIsland to follow-up sub-stages.
        Feature | EndCity | EndGateway | EndIsland | Fortress | Bastion => {
            unimplemented!(
                "Modern Overworld is_viable_structure_pos: {structure_type:?} not yet ported"
            )
        }
    }
}

/// Cubiomes' `g_monument_biomes1` — biome mask used by the
/// final ocean-coverage check (every cell in a 29-block radius
/// must be one of these).
const G_MONUMENT_BIOMES1: u64 = (1u64 << Biome::OCEAN.id())
    | (1u64 << Biome::DEEP_OCEAN.id())
    | (1u64 << Biome::RIVER.id())
    | (1u64 << Biome::FROZEN_RIVER.id())
    | (1u64 << Biome::FROZEN_OCEAN.id())
    | (1u64 << Biome::DEEP_FROZEN_OCEAN.id())
    | (1u64 << Biome::COLD_OCEAN.id())
    | (1u64 << Biome::DEEP_COLD_OCEAN.id())
    | (1u64 << Biome::LUKEWARM_OCEAN.id())
    | (1u64 << Biome::DEEP_LUKEWARM_OCEAN.id())
    | (1u64 << Biome::WARM_OCEAN.id())
    | (1u64 << Biome::DEEP_WARM_OCEAN.id());

/// 1.18+ Monument viability. Two gates:
///   1. The biome at the chunk centre (block-Y 36 ≈ ocean floor)
///      must be deep-ocean.
///   2. The 29-block radius around the chunk centre must be entirely
///      ocean/river (cubiomes' `g_monument_biomes1` mask).
fn viable_monument_modern(g: &Generator, chunk_x: i64, chunk_z: i64) -> bool {
    let sample_x = (chunk_x * 16 + 8) as i32;
    let sample_z = (chunk_z * 16 + 8) as i32;
    // (1) deep-ocean centre check at scale 4, y=36>>2.
    let id = g.biome_at(4, sample_x >> 2, 36 >> 2, sample_z >> 2).0;
    if !Biome::is_deep_ocean_id(id) {
        return false;
    }
    // (2) full ocean-mask check over the 29-radius area.
    are_biomes_viable_modern(g, sample_x, 63, sample_z, 29, G_MONUMENT_BIOMES1, 0)
}

/// 1.18+ subset of cubiomes' `areBiomesViable`. Samples a grid of
/// scale-4 biome cells around `(x, y, z)` extending `±rad/4` and
/// returns `true` iff every cell matches the `(valid_b, valid_m)`
/// biome mask.
fn are_biomes_viable_modern(
    g: &Generator,
    x: i32,
    y: i32,
    z: i32,
    rad: i32,
    valid_b: u64,
    valid_m: u64,
) -> bool {
    let x1 = (x - rad) >> 2;
    let x2 = (x + rad) >> 2;
    let sx = x2 - x1 + 1;
    let z1 = (z - rad) >> 2;
    let z2 = (z + rad) >> 2;
    let sz = z2 - z1 + 1;
    let y4 = (y - rad) >> 2;

    let bn = g
        .biome_noise
        .as_ref()
        .expect("are_biomes_viable_modern: modern Overworld must have BiomeNoise");

    // Check corners first; cubiomes also checks the full grid since
    // `approx == 0` is the default, so we do too.
    let corners = [(x1, z1), (x2, z2), (x1, z2), (x2, z1)];
    for (cx, cz) in corners {
        let id = g.biome_at(4, cx, y4, cz).0;
        if id < 0 || !crate::finder::locate_biome::id_matches(id, valid_b, valid_m) {
            return false;
        }
    }

    // Full grid — cubiomes uses the `dat` carry for MC-241546-style
    // order-dependent sampling.
    for i in 0..sx {
        let mut dat: u64 = 0;
        for j in 0..sz {
            let (id, _) = bn.sample_with_dat(x1 + i, y4, z1 + j, Some(&mut dat), 0);
            if id < 0 || !crate::finder::locate_biome::id_matches(id, valid_b, valid_m) {
                return false;
            }
        }
    }
    true
}

/// 1.18+ Outpost viability. Three gates:
///   1. `setAttemptSeed`-keyed `nextInt(5) == 0` per-chunk roll.
///   2. No Village placed within ±10 chunks (1.16.1+ short-circuit
///      bypasses the recursive `isViableStructurePos(Village)` check).
///   3. The biome at the chunkGenerateRnd-chosen variant centroid
///      matches `isViableFeatureBiome(Outpost, ...)`.
fn viable_outpost_modern(g: &Generator, x: i32, z: i32, chunk_x: i64, chunk_z: i64) -> bool {
    let mut rng = set_attempt_seed(g.seed, x >> 4, z >> 4);
    if rng.next_int(5) != 0 {
        return false;
    }
    let Some(vilconf) = get_structure_config(StructureType::Village, g.mc) else {
        return false;
    };
    let cx = (x >> 4) as i64;
    let cz = (z >> 4) as i64;
    let cx0 = cx - 10;
    let cx1 = cx + 10;
    let cz0 = cz - 10;
    let cz1 = cz + 10;
    let region = i32::from(vilconf.region_size);
    let rx0 = floordiv(cx0 as i32, region);
    let rx1 = floordiv(cx1 as i32, region);
    let rz0 = floordiv(cz0 as i32, region);
    let rz1 = floordiv(cz1 as i32, region);
    for rz in rz0..=rz1 {
        for rx in rx0..=rx1 {
            let p = get_feature_pos(vilconf, g.seed, rx, rz);
            let pc_x = (p.x >> 4) as i64;
            let pc_z = (p.z >> 4) as i64;
            if pc_x >= cx0 && pc_x <= cx1 && pc_z >= cz0 && pc_z <= cz1 {
                // 1.16.1+ short-circuits without the recursive check.
                return false;
            }
        }
    }

    // 1.18+ picks one of the 4 corners of the variant box.
    let mut rng2 = chunk_generate_rng(g.seed, x >> 4, z >> 4);
    let (off_x, off_z) = match rng2.next_int(4) {
        0 => (15, 15),
        1 => (-15, 15),
        2 => (-15, -15),
        3 => (15, -15),
        _ => return false,
    };
    let sample_x = (((chunk_x * 32 + off_x) / 2) >> 2) as i32;
    let sample_z = (((chunk_z * 32 + off_z) / 2) >> 2) as i32;
    let id = modern_biome_at_scale0(g, sample_x, 319 >> 2, sample_z);
    if id < 0 {
        return false;
    }
    is_viable_feature_biome(g.mc, StructureType::Outpost, id)
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
