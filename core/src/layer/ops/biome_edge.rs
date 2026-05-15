//! `mapBiomeEdge` — biome edge / family-aware replacement.
//!
//! Bit-exact port of cubiomes' `mapBiomeEdge`. The layer reads a
//! `(w+2, h+2)` parent and emits a `(w, h)` window. Cells whose ID
//! matches one of three "base" biomes (`wooded_badlands_plateau`,
//! `badlands_plateau`, `giant_tree_taiga`) are replaced with their
//! "edge" form unless all four cardinal neighbours are `are_similar`
//! to the base. Desert and swamp cells get further special-case
//! treatment.

#![allow(clippy::many_single_char_names)]

use crate::biome::Biome;
use crate::mc_version::MCVersion;

const DESERT: i32 = Biome::DESERT.id();
const SWAMP: i32 = Biome::SWAMP.id();
const PLAINS: i32 = Biome::PLAINS.id();
const JUNGLE: i32 = Biome::JUNGLE.id();
const JUNGLE_EDGE: i32 = Biome::JUNGLE_EDGE.id();
const BAMBOO_JUNGLE: i32 = Biome::BAMBOO_JUNGLE.id();
const SNOWY_TUNDRA: i32 = Biome::SNOWY_TUNDRA.id();
const SNOWY_TAIGA: i32 = Biome::SNOWY_TAIGA.id();
const WOODED_MOUNTAINS: i32 = Biome::WOODED_MOUNTAINS.id();
const BADLANDS: i32 = Biome::BADLANDS.id();
const WOODED_BADLANDS_PLATEAU: i32 = Biome::WOODED_BADLANDS_PLATEAU.id();
const BADLANDS_PLATEAU: i32 = Biome::BADLANDS_PLATEAU.id();
const GIANT_TREE_TAIGA: i32 = Biome::GIANT_TREE_TAIGA.id();
const TAIGA: i32 = Biome::TAIGA.id();

#[inline]
fn is_any4(target: i32, a: i32, b: i32, c: i32, d: i32) -> bool {
    target == a || target == b || target == c || target == d
}

/// If `id == base_id`, returns the edge-replaced value (or `id` itself
/// when every cardinal neighbour is similar to `base_id`). Returns
/// `None` if the cell shouldn't be replaced.
fn try_replace_edge(
    mc: MCVersion,
    v10: i32,
    v21: i32,
    v01: i32,
    v12: i32,
    id: i32,
    base_id: i32,
    edge_id: i32,
) -> Option<i32> {
    if id != base_id {
        return None;
    }
    let all_similar = Biome::are_similar_ids(mc, v10, base_id)
        && Biome::are_similar_ids(mc, v21, base_id)
        && Biome::are_similar_ids(mc, v01, base_id)
        && Biome::are_similar_ids(mc, v12, base_id);
    Some(if all_similar { id } else { edge_id })
}

/// `mapBiomeEdge` — parent `(w+2, h+2)`, output `(w, h)`.
pub fn map_biome_edge(mc: MCVersion, parent: &[Biome], out: &mut [Biome], w: usize, h: usize) {
    let p_w = w + 2;
    assert!(
        parent.len() >= p_w * (h + 2),
        "map_biome_edge: parent slice too small"
    );
    assert!(out.len() >= w * h, "map_biome_edge: output slice too small");

    for j in 0..h {
        for i in 0..w {
            let v11 = parent[(i + 1) + (j + 1) * p_w].id();
            let v10 = parent[(i + 1) + j * p_w].id();
            let v21 = parent[(i + 2) + (j + 1) * p_w].id();
            let v01 = parent[i + (j + 1) * p_w].id();
            let v12 = parent[(i + 1) + (j + 2) * p_w].id();

            let replaced = try_replace_edge(
                mc,
                v10,
                v21,
                v01,
                v12,
                v11,
                WOODED_BADLANDS_PLATEAU,
                BADLANDS,
            )
            .or_else(|| try_replace_edge(mc, v10, v21, v01, v12, v11, BADLANDS_PLATEAU, BADLANDS))
            .or_else(|| try_replace_edge(mc, v10, v21, v01, v12, v11, GIANT_TREE_TAIGA, TAIGA));

            let value = if let Some(v) = replaced {
                v
            } else if v11 == DESERT {
                if is_any4(SNOWY_TUNDRA, v10, v21, v01, v12) {
                    WOODED_MOUNTAINS
                } else {
                    v11
                }
            } else if v11 == SWAMP {
                if is_any4(DESERT, v10, v21, v01, v12)
                    || is_any4(SNOWY_TAIGA, v10, v21, v01, v12)
                    || is_any4(SNOWY_TUNDRA, v10, v21, v01, v12)
                {
                    PLAINS
                } else if is_any4(JUNGLE, v10, v21, v01, v12)
                    || is_any4(BAMBOO_JUNGLE, v10, v21, v01, v12)
                {
                    JUNGLE_EDGE
                } else {
                    v11
                }
            } else {
                v11
            };

            out[i + j * w] = Biome(value);
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
    fn uniform_forest_passes_through() {
        let parent = uniform_parent(Biome::FOREST.id(), 4, 4);
        let mut out = vec![Biome::NONE; 4 * 4];
        map_biome_edge(MCVersion::V1_18, &parent, &mut out, 4, 4);
        for cell in &out {
            assert_eq!(*cell, Biome::FOREST);
        }
    }

    #[test]
    fn uniform_wooded_badlands_plateau_stays_self() {
        // Surrounded by the same family, the replacement keeps `id`.
        let parent = uniform_parent(Biome::WOODED_BADLANDS_PLATEAU.id(), 4, 4);
        let mut out = vec![Biome::NONE; 4 * 4];
        map_biome_edge(MCVersion::V1_18, &parent, &mut out, 4, 4);
        for cell in &out {
            assert_eq!(*cell, Biome::WOODED_BADLANDS_PLATEAU);
        }
    }

    #[test]
    fn isolated_giant_tree_taiga_becomes_taiga_edge() {
        // Centre is giant_tree_taiga but all cardinals are forest.
        // are_similar(mc, forest, giant_tree_taiga) is false because the
        // categories differ, so the edge replacement fires.
        let mut parent = vec![Biome::FOREST; 3 * 3];
        parent[3 + 1] = Biome::GIANT_TREE_TAIGA;
        let mut out = vec![Biome::NONE; 1];
        map_biome_edge(MCVersion::V1_18, &parent, &mut out, 1, 1);
        assert_eq!(out[0], Biome::TAIGA);
    }

    #[test]
    fn desert_with_snowy_tundra_neighbour_becomes_wooded_mountains() {
        let mut parent = vec![Biome::FOREST; 3 * 3];
        parent[3 + 1] = Biome::DESERT; // centre
        parent[1] = Biome::SNOWY_TUNDRA; // (1, 0) = v10
        let mut out = vec![Biome::NONE; 1];
        map_biome_edge(MCVersion::V1_18, &parent, &mut out, 1, 1);
        assert_eq!(out[0], Biome::WOODED_MOUNTAINS);
    }

    #[test]
    fn swamp_with_jungle_neighbour_becomes_jungle_edge() {
        let mut parent = vec![Biome::FOREST; 3 * 3];
        parent[3 + 1] = Biome::SWAMP;
        parent[1] = Biome::JUNGLE;
        let mut out = vec![Biome::NONE; 1];
        map_biome_edge(MCVersion::V1_18, &parent, &mut out, 1, 1);
        assert_eq!(out[0], Biome::JUNGLE_EDGE);
    }

    #[test]
    fn swamp_with_desert_neighbour_becomes_plains() {
        let mut parent = vec![Biome::FOREST; 3 * 3];
        parent[3 + 1] = Biome::SWAMP;
        parent[1] = Biome::DESERT;
        let mut out = vec![Biome::NONE; 1];
        map_biome_edge(MCVersion::V1_18, &parent, &mut out, 1, 1);
        assert_eq!(out[0], Biome::PLAINS);
    }
}
