//! `mapHills` — biome / river-driven hill variant assignment.
//!
//! Bit-exact port of cubiomes' `mapHills`. Reads two parent grids:
//! the biome chain (`a11`) and the river chain (`b11`), both at
//! `(w+2, h+2)`. A river-side magic value `bn = (b11 - 2) mod 29` for
//! MC ≥ 1.7 selects between three behaviours:
//!
//! - `bn == 1` and `b11 >= 2` on non-shallow-ocean biome: pick the
//!   mutated variant of the biome (or keep the biome when no
//!   mutation exists).
//! - `bn == 0` or `mc_first_is_zero(cs, 3)`: pick a hill variant
//!   (e.g. forest -> `wooded_hills`) and only commit if at least
//!   3 (or 4 pre-1.7) of the four cardinal neighbours are similar
//!   to the centre biome.
//! - Otherwise: pass `a11` through unchanged.

#![allow(clippy::many_single_char_names, clippy::too_many_arguments)]

use crate::biome::Biome;
use crate::mc_version::MCVersion;
use crate::rng::{get_chunk_seed, mc_first_is_zero, mc_step_seed};

/// `mapHills` — biome parent (a) + river parent (b), each `(w+2, h+2)`.
#[allow(clippy::too_many_lines)]
pub fn map_hills(
    mc: MCVersion,
    start_salt: u64,
    start_seed: u64,
    biome_parent: &[Biome],
    river_parent: &[Biome],
    out: &mut [Biome],
    x: i32,
    z: i32,
    w: usize,
    h: usize,
) {
    let p_w = w + 2;
    assert!(
        biome_parent.len() >= p_w * (h + 2),
        "map_hills: biome parent slice too small"
    );
    assert!(
        river_parent.len() >= p_w * (h + 2),
        "map_hills: river parent slice too small"
    );
    assert!(out.len() >= w * h, "map_hills: output slice too small");

    let mc_ge_1_7 = mc.is_at_least(MCVersion::V1_7);
    let mc_le_1_6 = !mc_ge_1_7;

    for j in 0..h {
        for i in 0..w {
            let a11 = biome_parent[(i + 1) + (j + 1) * p_w].id();
            let b11 = river_parent[(i + 1) + (j + 1) * p_w].id();
            let idx = i + j * w;
            let bn = if mc_ge_1_7 { (b11 - 2) % 29 } else { -1 };

            // Branch 1: mutated biome.
            if bn == 1 && b11 >= 2 && !Biome::is_shallow_ocean_id(a11) {
                let m = Biome::get_mutated_id(mc, a11);
                out[idx] = Biome(if m > 0 { m } else { a11 });
                continue;
            }

            let mut cs = get_chunk_seed(start_seed, i as i32 + x, j as i32 + z);

            // Branch 2: hill variant pick (gated on 1/3 chance or
            // bn == 0).
            if bn == 0 || mc_first_is_zero(cs, 3) {
                let mut hill_id = a11;
                match a11 {
                    2 => hill_id = 17,  // desert -> desert_hills
                    4 => hill_id = 18,  // forest -> wooded_hills
                    27 => hill_id = 28, // birch_forest -> birch_forest_hills
                    29 => hill_id = 1,  // dark_forest -> plains
                    5 => hill_id = 19,  // taiga -> taiga_hills
                    32 => hill_id = 33, // giant_tree_taiga -> giant_tree_taiga_hills
                    30 => hill_id = 31, // snowy_taiga -> snowy_taiga_hills
                    1 => {
                        // plains: pre-1.7 -> forest, 1.7+ -> 1/3 wooded_hills, 2/3 forest.
                        if mc_le_1_6 {
                            hill_id = 4; // forest
                        } else {
                            cs = mc_step_seed(cs, start_salt);
                            hill_id = if mc_first_is_zero(cs, 3) { 18 } else { 4 };
                        }
                    }
                    12 => hill_id = 13,   // snowy_tundra -> snowy_mountains
                    21 => hill_id = 22,   // jungle -> jungle_hills
                    168 => hill_id = 169, // bamboo_jungle -> bamboo_jungle_hills
                    0 => {
                        if mc_ge_1_7 {
                            hill_id = 24; // ocean -> deep_ocean
                        }
                    }
                    3 => {
                        if mc_ge_1_7 {
                            hill_id = 34; // mountains -> wooded_mountains
                        }
                    }
                    35 => hill_id = 36, // savanna -> savanna_plateau
                    _ => {
                        if Biome::are_similar_ids(mc, a11, Biome::WOODED_BADLANDS_PLATEAU.id()) {
                            hill_id = Biome::BADLANDS.id();
                        } else if Biome::is_deep_ocean_id(a11) {
                            cs = mc_step_seed(cs, start_salt);
                            if mc_first_is_zero(cs, 3) {
                                cs = mc_step_seed(cs, start_salt);
                                hill_id = if mc_first_is_zero(cs, 2) {
                                    Biome::PLAINS.id()
                                } else {
                                    Biome::FOREST.id()
                                };
                            }
                        }
                    }
                }

                #[allow(clippy::collapsible_if)]
                if bn == 0 && hill_id != a11 {
                    let m = Biome::get_mutated_id(mc, hill_id);
                    hill_id = if m < 0 { a11 } else { m };
                }

                if hill_id == a11 {
                    out[idx] = Biome(a11);
                } else {
                    let a10 = biome_parent[(i + 1) + j * p_w].id();
                    let a21 = biome_parent[(i + 2) + (j + 1) * p_w].id();
                    let a01 = biome_parent[i + (j + 1) * p_w].id();
                    let a12 = biome_parent[(i + 1) + (j + 2) * p_w].id();
                    let mut equals = 0;
                    if Biome::are_similar_ids(mc, a10, a11) {
                        equals += 1;
                    }
                    if Biome::are_similar_ids(mc, a21, a11) {
                        equals += 1;
                    }
                    if Biome::are_similar_ids(mc, a01, a11) {
                        equals += 1;
                    }
                    if Biome::are_similar_ids(mc, a12, a11) {
                        equals += 1;
                    }
                    let threshold = if mc_le_1_6 { 4 } else { 3 };
                    out[idx] = Biome(if equals >= threshold { hill_id } else { a11 });
                }
            } else {
                out[idx] = Biome(a11);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uniform_parent(value: i32, w: usize, h: usize) -> Vec<Biome> {
        vec![Biome(value); (w + 2) * (h + 2)]
    }

    #[test]
    fn no_river_input_passes_biome_through() {
        // river_parent values of 0 -> bn = -1 (no branch fires) and the
        // pre-1.7 path falls through to a11 once the dice roll fails.
        // To make the test deterministic, run many cells and assert the
        // output is always one of {a11, wooded_hills} when biome is
        // FOREST.
        let biome = uniform_parent(Biome::FOREST.id(), 4, 4);
        let river = uniform_parent(0, 4, 4);
        let mut out = vec![Biome::NONE; 16];
        map_hills(MCVersion::V1_18, 1, 1, &biome, &river, &mut out, 0, 0, 4, 4);
        for cell in &out {
            assert!(
                *cell == Biome::FOREST || cell.id() == 18, // wooded_hills
                "unexpected map_hills output {cell:?}"
            );
        }
    }

    #[test]
    fn bn_1_with_river_promotes_to_mutated_when_available() {
        // b11 = 31 (river-parent value): (31 - 2) % 29 = 0, so this
        // cell hits the hill-pick branch, not the mutated branch.
        // For the mutated branch we want (b - 2) % 29 == 1, so b = 3.
        let biome = uniform_parent(Biome::FOREST.id(), 1, 1);
        let mut river = vec![Biome(0); 3 * 3];
        river[3 + 1] = Biome(3); // centre b11 = 3 -> bn = 1
        let mut out = vec![Biome::NONE; 1];
        map_hills(
            MCVersion::V1_18,
            42,
            42,
            &biome,
            &river,
            &mut out,
            0,
            0,
            1,
            1,
        );
        // FOREST -> mutated is flower_forest (132).
        assert_eq!(out[0].id(), 132);
    }

    #[test]
    fn ocean_centre_with_bn1_keeps_ocean() {
        // is_shallow_ocean(a11) blocks the mutated branch.
        let biome = uniform_parent(Biome::OCEAN.id(), 1, 1);
        let mut river = vec![Biome(0); 3 * 3];
        river[3 + 1] = Biome(3);
        let mut out = vec![Biome::NONE; 1];
        map_hills(MCVersion::V1_18, 7, 7, &biome, &river, &mut out, 0, 0, 1, 1);
        // Ocean centre with bn = 1 stays ocean (mutated branch is
        // blocked by is_shallow_ocean). The fall-through hill branch
        // may pick deep_ocean if the 1/3 chunk seed fires; otherwise
        // it stays ocean.
        assert!(out[0] == Biome::OCEAN || out[0] == Biome::DEEP_OCEAN);
    }
}
