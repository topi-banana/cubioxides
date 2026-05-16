//! `canBiomeGenerate(layerId, mc, flags, id)` — return `true` when
//! the biome `id` is allowed to appear at the requested layer for
//! the given MC version. Bit-exact port of cubiomes' helper of the
//! same name.
//!
//! Used by cubiomes' upstream `setupBiomeFilter` /
//! `getAvailableBiomes` to decide which biomes a layer can possibly
//! output. The cubioxides port of those functions is a follow-up.
//! Cubiomes prints a warning and
//! returns 0 for unsupported layers; we panic instead to surface
//! the misuse earlier.

#![allow(clippy::missing_panics_doc, clippy::doc_markdown)]

use crate::biome::Biome;
use crate::generator::FORCE_OCEAN_VARIANTS;
use crate::layer::LayerId;
use crate::mc_version::MCVersion;

const BAMBOO_JUNGLE: i32 = 168;

/// `canBiomeGenerate(layerId, mc, flags, id)` — return `true` iff
/// the biome `id` can be produced by `layerId`. Mirrors cubiomes'
/// per-layer filter cascade.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn can_biome_generate(layer: LayerId, mc: MCVersion, flags: u32, id: i32) -> bool {
    let mut dofilter = false;

    if mc.is_at_least(MCVersion::V1_13) {
        if layer == LayerId::OceanTemp256 {
            return Biome::is_shallow_ocean_id(id);
        }
        if (flags & FORCE_OCEAN_VARIANTS) != 0 && Biome::is_oceanic_id(id) {
            return id != Biome::DEEP_WARM_OCEAN.id();
        }
    }

    if dofilter || layer == LayerId::Biome256 {
        dofilter = true;
        if id >= 64 {
            return false;
        }
    }
    if dofilter || (layer == LayerId::Bamboo256 && mc.is_at_least(MCVersion::V1_14)) {
        dofilter = true;
        if matches!(
            id,
            23 /* jungle_edge */ | 34 /* wooded_mountains */ | 37 /* badlands */
        ) {
            return false;
        }
    }
    if dofilter || (layer == LayerId::BiomeEdge64 && mc.is_at_least(MCVersion::V1_7)) {
        dofilter = true;
        if id >= 64 && id != BAMBOO_JUNGLE {
            return false;
        }
        if matches!(
            id,
            13 // snowy_mountains
            | 17 // desert_hills
            | 18 // wooded_hills
            | 19 // taiga_hills
            | 22 // jungle_hills
            | 28 // birch_forest_hills
            | 31 // snowy_taiga_hills
            | 33 // giant_tree_taiga_hills
            | 36 // savanna_plateau
        ) {
            return false;
        }
    }
    if dofilter || (layer == LayerId::Zoom64 && !mc.is_at_least(MCVersion::V1_1)) {
        // cubiomes: layerId == L_ZOOM_64 && mc <= MC_1_0
        dofilter = true;
        if id == 15
        /* mushroom_field_shore */
        {
            return false;
        }
    }
    if dofilter || layer == LayerId::Hills64 {
        dofilter = true;
        if id == Biome::FROZEN_OCEAN.id() {
            return false;
        }
    }
    if dofilter || (layer == LayerId::Zoom16 && !mc.is_at_least(MCVersion::V1_7)) {
        // cubiomes: layerId == L_ZOOM_16 && mc <= MC_1_6
        dofilter = true;
        if id == 20
        /* mountain_edge */
        {
            return false;
        }
    }
    if dofilter || (layer == LayerId::Sunflower64 && mc.is_at_least(MCVersion::V1_7)) {
        dofilter = true;
        match id {
            16 /* beach */ | 25 /* stone_shore */ | 26 /* snowy_beach */ => return false,
            15 /* mushroom_field_shore */ if mc != MCVersion::V1_0 => return false,
            _ => {}
        }
    }
    if dofilter || layer == LayerId::Shore16 {
        dofilter = true;
        if id == Biome::RIVER.id() {
            return false;
        }
    }
    if dofilter || (layer == LayerId::SwampRiver16 && !mc.is_at_least(MCVersion::V1_7)) {
        // cubiomes: layerId == L_SWAMP_RIVER_16 && mc <= MC_1_6
        dofilter = true;
        if id == Biome::FROZEN_RIVER.id() {
            return false;
        }
    }
    if dofilter || layer == LayerId::RiverMix4 {
        dofilter = true;
        if Biome::is_deep_ocean_id(id) && id != Biome::DEEP_OCEAN.id() {
            return false;
        }
        if Biome::is_shallow_ocean_id(id) && id != Biome::OCEAN.id() {
            // For mc >= 1.7 or id != frozen_ocean → reject.
            if mc.is_at_least(MCVersion::V1_7) || id != Biome::FROZEN_OCEAN.id() {
                return false;
            }
        }
    }
    if dofilter || (layer == LayerId::OceanMix4 && mc.is_at_least(MCVersion::V1_13)) {
        dofilter = true;
    }

    if !dofilter && layer != LayerId::Voronoi1 {
        // Cubiomes prints a warning to stderr and returns 0 for
        // unsupported (layer, mc) combinations; we silently return
        // `false` to match the bit-exact answer without flooding
        // stderr. Callers should constrain `layer` to the set that
        // their `mc` supports.
        return false;
    }
    Biome::is_overworld_id(mc, id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ocean_temp_256_only_shallow() {
        assert!(can_biome_generate(
            LayerId::OceanTemp256,
            MCVersion::V1_18,
            0,
            Biome::OCEAN.id()
        ));
        assert!(!can_biome_generate(
            LayerId::OceanTemp256,
            MCVersion::V1_18,
            0,
            Biome::DEEP_OCEAN.id()
        ));
    }

    #[test]
    fn biome_256_rejects_id_64_plus() {
        // bamboo_jungle is 168, > 64, so rejected at L_BIOME_256.
        assert!(!can_biome_generate(
            LayerId::Biome256,
            MCVersion::V1_18,
            0,
            BAMBOO_JUNGLE
        ));
        // plains (1) is allowed.
        assert!(can_biome_generate(
            LayerId::Biome256,
            MCVersion::V1_18,
            0,
            Biome::PLAINS.id()
        ));
    }

    #[test]
    fn voronoi_1_passes_through_overworld_check() {
        // Voronoi1 is the "default" arm — no filter, just isOverworld.
        assert!(can_biome_generate(
            LayerId::Voronoi1,
            MCVersion::V1_18,
            0,
            Biome::PLAINS.id()
        ));
    }
}
