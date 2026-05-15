//! `mapRiverMix` — overlay river cells onto a biome map.
//!
//! Bit-exact port of cubiomes' `mapRiverMix`. The layer takes two
//! parent grids (biome chain + river chain) and emits the merged
//! result: river cells override land biomes (with `snowy_tundra ->
//! frozen_river` and `mushroom_* -> mushroom_field_shore` special
//! cases), but never overwrite ocean / oceanic biomes (the latter
//! check is gated on `mc >= MC_1_7`).

use crate::biome::Biome;
use crate::mc_version::MCVersion;

const OCEAN: i32 = Biome::OCEAN.id();
const RIVER: i32 = Biome::RIVER.id();
const FROZEN_RIVER: i32 = Biome::FROZEN_RIVER.id();
const SNOWY_TUNDRA: i32 = Biome::SNOWY_TUNDRA.id();
const MUSHROOM_FIELDS: i32 = Biome::MUSHROOM_FIELDS.id();
const MUSHROOM_FIELD_SHORE: i32 = Biome::MUSHROOM_FIELD_SHORE.id();

/// `mapRiverMix` — merge `biome_in[i]` and `river_in[i]` into `out[i]`
/// over a `(w, h)` window.
///
/// Both parents are size `w * h`.
#[allow(clippy::too_many_arguments)]
pub fn map_river_mix(
    mc: MCVersion,
    biome_in: &[Biome],
    river_in: &[Biome],
    out: &mut [Biome],
    w: usize,
    h: usize,
) {
    let len = w * h;
    assert!(
        biome_in.len() >= len,
        "map_river_mix: biome parent slice too small"
    );
    assert!(
        river_in.len() >= len,
        "map_river_mix: river parent slice too small"
    );
    assert!(out.len() >= len, "map_river_mix: output slice too small");

    let mc_le_1_6 = !mc.is_at_least(MCVersion::V1_7);

    for idx in 0..len {
        let mut v = biome_in[idx].id();
        let river_id = river_in[idx].id();
        if river_id == RIVER && v != OCEAN && (mc_le_1_6 || !Biome::is_oceanic_id(v)) {
            v = if v == SNOWY_TUNDRA {
                FROZEN_RIVER
            } else if v == MUSHROOM_FIELDS || v == MUSHROOM_FIELD_SHORE {
                MUSHROOM_FIELD_SHORE
            } else {
                RIVER
            };
        }
        out[idx] = Biome(v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_river_passes_through() {
        let biome = vec![Biome::FOREST; 16];
        let river = vec![Biome::NONE; 16];
        let mut out = vec![Biome::OCEAN; 16];
        map_river_mix(MCVersion::V1_18, &biome, &river, &mut out, 4, 4);
        for cell in &out {
            assert_eq!(*cell, Biome::FOREST);
        }
    }

    #[test]
    fn snowy_tundra_becomes_frozen_river() {
        let biome = vec![Biome::SNOWY_TUNDRA; 16];
        let river = vec![Biome::RIVER; 16];
        let mut out = vec![Biome::NONE; 16];
        map_river_mix(MCVersion::V1_18, &biome, &river, &mut out, 4, 4);
        for cell in &out {
            assert_eq!(*cell, Biome::FROZEN_RIVER);
        }
    }

    #[test]
    fn mushroom_fields_becomes_shore() {
        let biome = vec![Biome::MUSHROOM_FIELDS; 16];
        let river = vec![Biome::RIVER; 16];
        let mut out = vec![Biome::NONE; 16];
        map_river_mix(MCVersion::V1_18, &biome, &river, &mut out, 4, 4);
        for cell in &out {
            assert_eq!(*cell, Biome::MUSHROOM_FIELD_SHORE);
        }
    }

    #[test]
    fn ocean_is_not_overridden_post_1_7() {
        let biome = vec![Biome::OCEAN; 16];
        let river = vec![Biome::RIVER; 16];
        let mut out = vec![Biome::NONE; 16];
        map_river_mix(MCVersion::V1_18, &biome, &river, &mut out, 4, 4);
        for cell in &out {
            assert_eq!(*cell, Biome::OCEAN);
        }
    }

    #[test]
    fn warm_ocean_is_not_overridden_post_1_7() {
        // For 1.7+, isOceanic includes warm_ocean, so river overlay must skip it.
        let biome = vec![Biome::WARM_OCEAN; 16];
        let river = vec![Biome::RIVER; 16];
        let mut out = vec![Biome::NONE; 16];
        map_river_mix(MCVersion::V1_18, &biome, &river, &mut out, 4, 4);
        for cell in &out {
            assert_eq!(*cell, Biome::WARM_OCEAN);
        }
    }

    #[test]
    fn warm_ocean_is_overridden_pre_1_7() {
        // Pre-1.7 only `ocean` (id 0) is protected from rivers.
        let biome = vec![Biome::WARM_OCEAN; 16];
        let river = vec![Biome::RIVER; 16];
        let mut out = vec![Biome::NONE; 16];
        map_river_mix(MCVersion::V1_6, &biome, &river, &mut out, 4, 4);
        for cell in &out {
            assert_eq!(*cell, Biome::RIVER);
        }
    }
}
