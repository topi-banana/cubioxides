//! `getAvailableBiomes(mL, mM, layerId, mc, flags)` — return the
//! set of biome IDs that can possibly appear at the requested
//! layer for the given MC version.
//!
//! Built on top of [`crate::finder::can_biome_generate::can_biome_generate`].
//! Mirrors cubiomes' helper of the same name in spirit; for the
//! pre-1.18 layered branch we pass the loop variable as `id` and
//! the input `flags` as `flags`, which is the obvious intent —
//! cubiomes itself has those two arguments swapped (a known
//! upstream bug we do NOT replicate).

#![allow(clippy::missing_panics_doc, clippy::doc_markdown)]

use crate::biome::Biome;
use crate::biome_set::BiomeSet;
use crate::finder::can_biome_generate::can_biome_generate;
use crate::layer::LayerId;
use crate::mc_version::MCVersion;

/// `getAvailableBiomes` — fill a [`BiomeSet`] with the biome IDs
/// that can possibly appear at `layer` for `mc` and `flags`.
///
/// **Cubiomes parity note**: cubiomes' upstream implementation calls
/// `canBiomeGenerate(layerId, mc, i, flags)` — passing the loop
/// variable `i` (a biome id) as the `flags` argument and the input
/// `flags` (a bitfield) as the `id` argument. That is an argument-
/// swap bug. We deliberately call with arguments in the documented
/// order so the result is meaningful. Callers that need cubiomes'
/// (buggy) bitset for parity should use the raw
/// [`can_biome_generate`] in the same swapped form themselves.
///
/// # Example
///
/// ```
/// use cubioxides::MCVersion;
/// use cubioxides::biome::Biome;
/// use cubioxides::finder::get_available_biomes;
/// use cubioxides::layer::LayerId;
///
/// // 1.13–1.17 at the 1:256 ocean-temperature layer always emits
/// // only the five ocean temperature variants — a deterministic
/// // invariant of the layer stack that's useful for filter
/// // pre-screening. (1.18+ bypasses layer filtering entirely.)
/// let set = get_available_biomes(LayerId::OceanTemp256, MCVersion::V1_16_1, 0);
/// assert!(set.contains(Biome::WARM_OCEAN.id()));
/// assert!(set.contains(Biome::FROZEN_OCEAN.id()));
/// assert!(!set.contains(Biome::PLAINS.id()));
/// ```
#[must_use]
pub fn get_available_biomes(layer: LayerId, mc: MCVersion, flags: u32) -> BiomeSet {
    let mut set = BiomeSet::new();

    // B1.7 and 1.18+ skip the layer filter entirely; every overworld
    // biome is "available" (the actual biome generation pipeline
    // doesn't use layer filtering in these branches).
    if !mc.is_at_least(MCVersion::V1_0) || mc.is_at_least(MCVersion::V1_18) {
        for i in 0_i32..64 {
            if Biome::is_overworld_id(mc, i) {
                set.m_low |= 1u64 << i;
            }
            if Biome::is_overworld_id(mc, i + 128) {
                set.m_mut |= 1u64 << i;
            }
        }
        return set;
    }

    // 1.13+ special-case: `L_OCEAN_TEMP_256` only emits the five
    // ocean temperature variants.
    if mc.is_at_least(MCVersion::V1_13) && layer == LayerId::OceanTemp256 {
        set.m_low = (1u64 << Biome::OCEAN.id())
            | (1u64 << Biome::FROZEN_OCEAN.id())
            | (1u64 << Biome::WARM_OCEAN.id())
            | (1u64 << Biome::LUKEWARM_OCEAN.id())
            | (1u64 << Biome::COLD_OCEAN.id());
        return set;
    }

    // Layered branch — query `can_biome_generate` per ID. Note the
    // argument order (id, flags) — see the doc comment about the
    // cubiomes upstream arg-swap bug.
    for i in 0_i32..64 {
        if can_biome_generate(layer, mc, flags, i) {
            set.m_low |= 1u64 << i;
        }
        if can_biome_generate(layer, mc, flags, i + 128) {
            set.m_mut |= 1u64 << i;
        }
    }
    set
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ocean_temp_256_only_oceans_at_1_13_plus() {
        let s = get_available_biomes(LayerId::OceanTemp256, MCVersion::V1_18, 0);
        // 1.18+ takes the "all overworld" branch, not the special.
        // Let me instead test V1_13 where the special applies.
        let s13 = get_available_biomes(LayerId::OceanTemp256, MCVersion::V1_13, 0);
        let expected: u64 = (1u64 << Biome::OCEAN.id())
            | (1u64 << Biome::FROZEN_OCEAN.id())
            | (1u64 << Biome::WARM_OCEAN.id())
            | (1u64 << Biome::LUKEWARM_OCEAN.id())
            | (1u64 << Biome::COLD_OCEAN.id());
        assert_eq!(s13.m_low, expected);
        assert_eq!(s13.m_mut, 0);
        let _ = s;
    }

    #[test]
    fn voronoi_1_includes_all_overworld_at_1_18() {
        // 1.18+ "skip filter" branch — every overworld biome.
        let s = get_available_biomes(LayerId::Voronoi1, MCVersion::V1_18, 0);
        // Plains (1) is overworld.
        assert!((s.m_low & (1u64 << 1)) != 0);
        // Sunflower plains (129) is overworld + mutated.
        assert!((s.m_mut & (1u64 << (129 - 128))) != 0);
    }

    #[test]
    fn biome_256_layered_excludes_high_ids() {
        // For V1_17 with Biome256 we should NOT see ID >= 64 since
        // `can_biome_generate` rejects them at the Biome256 step.
        let s = get_available_biomes(LayerId::Biome256, MCVersion::V1_17, 0);
        // No high-id (mutated) biome should be set.
        assert_eq!(s.m_mut, 0);
        // Plains is allowed.
        assert!((s.m_low & (1u64 << 1)) != 0);
    }
}
