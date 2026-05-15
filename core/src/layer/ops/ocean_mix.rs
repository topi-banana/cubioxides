//! `mapOceanMix` — combine the biome chain output with the ocean
//! chain output.
//!
//! Bit-exact port of cubiomes' `mapOceanMix`. Reads two parent grids:
//!
//! - `ocean_parent` is the output of `map_ocean_temp` (warm /
//!   lukewarm / ocean / cold / frozen), sized `(w, h)`.
//! - `biome_parent` is the biome chain, sized `(w + 16, h + 16)` at
//!   origin `(x - 8, z - 8)`. cubiomes computes the exact rectangle
//!   needed from the warm / frozen ocean cells; this port asks the
//!   caller for the bounding box that always works.
//!
//! For each cell:
//! - When the biome chain says "land" (non-oceanic), the land biome
//!   wins.
//! - When the biome chain is oceanic and the ocean cell is warm /
//!   frozen *but* any cell within an 8-cell radius (step 4) on the
//!   biome side is non-oceanic, demote to lukewarm / cold respectively.
//! - Otherwise, when the biome chain says `deep_ocean`, promote the
//!   ocean variant to its deep counterpart.

#![allow(clippy::many_single_char_names)]

use crate::biome::Biome;

const OCEAN: i32 = Biome::OCEAN.id();
const WARM_OCEAN: i32 = Biome::WARM_OCEAN.id();
const LUKEWARM_OCEAN: i32 = Biome::LUKEWARM_OCEAN.id();
const COLD_OCEAN: i32 = Biome::COLD_OCEAN.id();
const FROZEN_OCEAN: i32 = Biome::FROZEN_OCEAN.id();
const DEEP_OCEAN: i32 = Biome::DEEP_OCEAN.id();
const DEEP_LUKEWARM_OCEAN: i32 = Biome::DEEP_LUKEWARM_OCEAN.id();
const DEEP_COLD_OCEAN: i32 = Biome::DEEP_COLD_OCEAN.id();
const DEEP_FROZEN_OCEAN: i32 = Biome::DEEP_FROZEN_OCEAN.id();

/// `mapOceanMix` — merge ocean variant with land biome chain.
///
/// Both parents are addressed in the same coordinate frame:
/// `ocean_parent[i, j]` covers cell `(x + i, z + j)`, and
/// `biome_parent[i + 8, j + 8]` covers the same cell (the biome
/// parent's `(0, 0)` is offset by `-8, -8` to give the 8-cell radius
/// the lookup below requires).
pub fn map_ocean_mix(
    ocean_parent: &[Biome],
    biome_parent: &[Biome],
    out: &mut [Biome],
    w: usize,
    h: usize,
) {
    assert!(
        ocean_parent.len() >= w * h,
        "map_ocean_mix: ocean parent slice too small"
    );
    let biome_w = w + 16;
    let biome_h = h + 16;
    assert!(
        biome_parent.len() >= biome_w * biome_h,
        "map_ocean_mix: biome parent slice too small (need (w+16) * (h+16))"
    );
    assert!(out.len() >= w * h, "map_ocean_mix: output slice too small");

    for j in 0..h {
        for i in 0..w {
            let idx = i + j * w;
            let land_id = biome_parent[(i + 8) + (j + 8) * biome_w].id();
            let ocean_id = ocean_parent[idx].id();

            if !Biome::is_oceanic_id(land_id) {
                out[idx] = Biome(land_id);
                continue;
            }

            let replace_id = if ocean_id == WARM_OCEAN {
                LUKEWARM_OCEAN
            } else if ocean_id == FROZEN_OCEAN {
                COLD_OCEAN
            } else {
                0
            };

            if replace_id != 0 && nearby_has_land(biome_parent, biome_w, i, j) {
                out[idx] = Biome(replace_id);
                continue;
            }

            let result = if land_id == DEEP_OCEAN {
                match ocean_id {
                    LUKEWARM_OCEAN => DEEP_LUKEWARM_OCEAN,
                    OCEAN => DEEP_OCEAN,
                    COLD_OCEAN => DEEP_COLD_OCEAN,
                    FROZEN_OCEAN => DEEP_FROZEN_OCEAN,
                    _ => ocean_id,
                }
            } else {
                ocean_id
            };
            out[idx] = Biome(result);
        }
    }
}

/// Returns `true` iff at least one biome cell within 8 cells (step 4)
/// of the centre is non-oceanic. The 5x5 sample grid matches cubiomes'
/// `for (ii = -8; ii <= 8; ii += 4)` loops.
fn nearby_has_land(biome_parent: &[Biome], biome_w: usize, i: usize, j: usize) -> bool {
    for ij in (0..=16).step_by(4) {
        for ii in (0..=16).step_by(4) {
            // ii, ij run from 0..=16 inclusive in steps of 4. Add to
            // (i, j) to get the biome-parent coordinates (already
            // offset by 8 in both axes).
            let id = biome_parent[(i + ii) + (j + ij) * biome_w].id();
            if !Biome::is_oceanic_id(id) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uniform_biome_parent(value: i32, w: usize, h: usize) -> Vec<Biome> {
        vec![Biome(value); (w + 16) * (h + 16)]
    }

    #[test]
    fn full_ocean_input_passes_ocean_through() {
        let ocean = vec![Biome::OCEAN; 16];
        let biome = uniform_biome_parent(Biome::OCEAN.id(), 4, 4);
        let mut out = vec![Biome::NONE; 16];
        map_ocean_mix(&ocean, &biome, &mut out, 4, 4);
        for cell in &out {
            assert_eq!(*cell, Biome::OCEAN);
        }
    }

    #[test]
    fn non_oceanic_biome_overrides_ocean() {
        let ocean = vec![Biome::WARM_OCEAN; 16];
        let biome = uniform_biome_parent(Biome::FOREST.id(), 4, 4);
        let mut out = vec![Biome::NONE; 16];
        map_ocean_mix(&ocean, &biome, &mut out, 4, 4);
        for cell in &out {
            assert_eq!(*cell, Biome::FOREST);
        }
    }

    #[test]
    fn warm_ocean_with_land_nearby_becomes_lukewarm() {
        let ocean = vec![Biome::WARM_OCEAN; 1];
        // 17x17 biome parent: everything ocean except one cell, which
        // sits within the 8-cell sample grid of the centre.
        let mut biome = vec![Biome::OCEAN; 17 * 17];
        biome[12 * 17 + 12] = Biome::FOREST; // offset (4, 4) from centre (8, 8)
        let mut out = vec![Biome::NONE; 1];
        map_ocean_mix(&ocean, &biome, &mut out, 1, 1);
        assert_eq!(out[0], Biome::LUKEWARM_OCEAN);
    }

    #[test]
    fn warm_ocean_with_no_land_nearby_stays_warm() {
        let ocean = vec![Biome::WARM_OCEAN; 1];
        let biome = uniform_biome_parent(Biome::OCEAN.id(), 1, 1);
        let mut out = vec![Biome::NONE; 1];
        map_ocean_mix(&ocean, &biome, &mut out, 1, 1);
        assert_eq!(out[0], Biome::WARM_OCEAN);
    }

    #[test]
    fn frozen_ocean_with_land_nearby_becomes_cold() {
        let ocean = vec![Biome::FROZEN_OCEAN; 1];
        let mut biome = vec![Biome::OCEAN; 17 * 17];
        biome[8 * 17 + 12] = Biome::FOREST;
        let mut out = vec![Biome::NONE; 1];
        map_ocean_mix(&ocean, &biome, &mut out, 1, 1);
        assert_eq!(out[0], Biome::COLD_OCEAN);
    }

    #[test]
    fn deep_ocean_centre_promotes_lukewarm_to_deep_lukewarm() {
        let ocean = vec![Biome::LUKEWARM_OCEAN; 1];
        let biome = uniform_biome_parent(Biome::DEEP_OCEAN.id(), 1, 1);
        let mut out = vec![Biome::NONE; 1];
        map_ocean_mix(&ocean, &biome, &mut out, 1, 1);
        assert_eq!(out[0], Biome::DEEP_LUKEWARM_OCEAN);
    }

    #[test]
    fn deep_ocean_centre_promotes_ocean_to_deep_ocean() {
        let ocean = vec![Biome::OCEAN; 1];
        let biome = uniform_biome_parent(Biome::DEEP_OCEAN.id(), 1, 1);
        let mut out = vec![Biome::NONE; 1];
        map_ocean_mix(&ocean, &biome, &mut out, 1, 1);
        assert_eq!(out[0], Biome::DEEP_OCEAN);
    }
}
