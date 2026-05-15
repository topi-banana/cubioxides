//! `mapShore` — biome shore replacement.
//!
//! Bit-exact port of cubiomes' `mapShore`. Reads a `(w+2, h+2)`
//! parent rectangle and emits a `(w, h)` window. The layer dispatches
//! on the centre biome:
//!
//! - `mushroom_fields` adjacent to any ocean -> `mushroom_field_shore`.
//! - MC ≤ 1.0 passes the centre through unchanged.
//! - MC ≤ 1.6 promotes isolated mountains to `mountain_edge` and
//!   converts non-river / non-swamp / non-ocean cells adjacent to
//!   ocean into `beach`.
//! - MC ≥ 1.7 applies the modern shore rules: jungle is wrapped in
//!   `jungle_edge` unless surrounded by jungle / forest / taiga /
//!   ocean; mountains and `wooded_mountains` become `stone_shore`
//!   when next to ocean; snowy biomes become `snowy_beach`; badlands
//!   variants become `desert` unless surrounded by mesa or adjacent
//!   to ocean; everything else becomes `beach` when next to ocean.

#![allow(clippy::many_single_char_names)]

use crate::biome::Biome;
use crate::mc_version::MCVersion;

const OCEAN: i32 = Biome::OCEAN.id();
const DEEP_OCEAN: i32 = Biome::DEEP_OCEAN.id();
const RIVER: i32 = Biome::RIVER.id();
const SWAMP: i32 = Biome::SWAMP.id();
const BEACH: i32 = Biome::BEACH.id();
const STONE_SHORE: i32 = Biome::STONE_SHORE.id();
const SNOWY_BEACH: i32 = Biome::SNOWY_BEACH.id();
const DESERT: i32 = Biome::DESERT.id();
const MUSHROOM_FIELDS: i32 = Biome::MUSHROOM_FIELDS.id();
const MUSHROOM_FIELD_SHORE: i32 = Biome::MUSHROOM_FIELD_SHORE.id();
const MOUNTAINS: i32 = Biome::MOUNTAINS.id();
const WOODED_MOUNTAINS: i32 = Biome::WOODED_MOUNTAINS.id();
const MOUNTAIN_EDGE: i32 = Biome::MOUNTAIN_EDGE.id();
const BADLANDS: i32 = Biome::BADLANDS.id();
const WOODED_BADLANDS_PLATEAU: i32 = Biome::WOODED_BADLANDS_PLATEAU.id();
const JUNGLE: i32 = Biome::JUNGLE.id();
const JUNGLE_EDGE: i32 = Biome::JUNGLE_EDGE.id();
const FOREST: i32 = Biome::FOREST.id();
const TAIGA: i32 = Biome::TAIGA.id();

#[inline]
fn is_any4(target: i32, a: i32, b: i32, c: i32, d: i32) -> bool {
    target == a || target == b || target == c || target == d
}

#[inline]
fn is_any4_oceanic(a: i32, b: i32, c: i32, d: i32) -> bool {
    Biome::is_oceanic_id(a)
        || Biome::is_oceanic_id(b)
        || Biome::is_oceanic_id(c)
        || Biome::is_oceanic_id(d)
}

/// Replace `id` with `replace_id` when any cardinal neighbour is
/// oceanic. Mirrors cubiomes' inline `replaceOcean`. Returns `false`
/// when `id` itself is oceanic (in which case the centre stays).
#[inline]
fn replace_ocean(
    out: &mut [Biome],
    idx: usize,
    v10: i32,
    v21: i32,
    v01: i32,
    v12: i32,
    id: i32,
    replace_id: i32,
) -> bool {
    if Biome::is_oceanic_id(id) {
        return false;
    }
    out[idx] = Biome(if is_any4_oceanic(v10, v21, v01, v12) {
        replace_id
    } else {
        id
    });
    true
}

/// `true` if all four neighbours are jungle-family, forest, taiga, or
/// oceanic. Mirrors cubiomes' inline `isAll4JFTO`.
#[inline]
fn is_all4_jfto(mc: MCVersion, a: i32, b: i32, c: i32, d: i32) -> bool {
    let check = |id: i32| {
        Biome::get_category_id(mc, id) == JUNGLE
            || id == FOREST
            || id == TAIGA
            || Biome::is_oceanic_id(id)
    };
    check(a) && check(b) && check(c) && check(d)
}

/// `mapShore` — parent `(w+2, h+2)`, output `(w, h)`.
#[allow(clippy::too_many_arguments)]
pub fn map_shore(mc: MCVersion, parent: &[Biome], out: &mut [Biome], w: usize, h: usize) {
    let p_w = w + 2;
    assert!(
        parent.len() >= p_w * (h + 2),
        "map_shore: parent slice too small"
    );
    assert!(out.len() >= w * h, "map_shore: output slice too small");

    let mc_le_1_0 = !mc.is_at_least(MCVersion::V1_1);
    let mc_le_1_6 = !mc.is_at_least(MCVersion::V1_7);

    for j in 0..h {
        for i in 0..w {
            let v11 = parent[(i + 1) + (j + 1) * p_w].id();
            let v10 = parent[(i + 1) + j * p_w].id();
            let v21 = parent[(i + 2) + (j + 1) * p_w].id();
            let v01 = parent[i + (j + 1) * p_w].id();
            let v12 = parent[(i + 1) + (j + 2) * p_w].id();
            let idx = i + j * w;

            // Mushroom shore handling is shared across every version.
            if v11 == MUSHROOM_FIELDS {
                out[idx] = Biome(if is_any4(OCEAN, v10, v21, v01, v12) {
                    MUSHROOM_FIELD_SHORE
                } else {
                    v11
                });
                continue;
            }

            if mc_le_1_0 {
                out[idx] = Biome(v11);
                continue;
            }

            if mc_le_1_6 {
                let mut v = v11;
                if v11 == MOUNTAINS {
                    if v10 != MOUNTAINS || v21 != MOUNTAINS || v01 != MOUNTAINS || v12 != MOUNTAINS
                    {
                        v = MOUNTAIN_EDGE;
                    }
                } else if v11 != OCEAN
                    && v11 != RIVER
                    && v11 != SWAMP
                    && is_any4(OCEAN, v10, v21, v01, v12)
                {
                    v = BEACH;
                }
                out[idx] = Biome(v);
                continue;
            }

            // 1.7+: full shore dispatch.
            if Biome::get_category_id(mc, v11) == JUNGLE {
                if is_all4_jfto(mc, v10, v21, v01, v12) {
                    out[idx] = Biome(if is_any4_oceanic(v10, v21, v01, v12) {
                        BEACH
                    } else {
                        v11
                    });
                } else {
                    out[idx] = Biome(JUNGLE_EDGE);
                }
            } else if v11 == MOUNTAINS || v11 == WOODED_MOUNTAINS {
                replace_ocean(out, idx, v10, v21, v01, v12, v11, STONE_SHORE);
            } else if Biome::is_snowy_id(v11) {
                replace_ocean(out, idx, v10, v21, v01, v12, v11, SNOWY_BEACH);
            } else if v11 == BADLANDS || v11 == WOODED_BADLANDS_PLATEAU {
                // Matches cubiomes' nested if/else structure verbatim;
                // the two `Biome(v11)` arms are intentional duplicates
                // of cubiomes' separate code paths.
                #[allow(clippy::if_same_then_else)]
                let value = if is_any4_oceanic(v10, v21, v01, v12) {
                    v11
                } else if Biome::is_mesa_id(v10)
                    && Biome::is_mesa_id(v21)
                    && Biome::is_mesa_id(v01)
                    && Biome::is_mesa_id(v12)
                {
                    v11
                } else {
                    DESERT
                };
                out[idx] = Biome(value);
            } else if v11 != OCEAN && v11 != DEEP_OCEAN && v11 != RIVER && v11 != SWAMP {
                out[idx] = Biome(if is_any4_oceanic(v10, v21, v01, v12) {
                    BEACH
                } else {
                    v11
                });
            } else {
                out[idx] = Biome(v11);
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
    fn uniform_forest_passes_through_1_18() {
        let parent = uniform_parent(Biome::FOREST.id(), 4, 4);
        let mut out = vec![Biome::NONE; 4 * 4];
        map_shore(MCVersion::V1_18, &parent, &mut out, 4, 4);
        for cell in &out {
            assert_eq!(*cell, Biome::FOREST);
        }
    }

    #[test]
    fn forest_next_to_ocean_becomes_beach_1_18() {
        let mut parent = vec![Biome::OCEAN; 3 * 3];
        parent[3 + 1] = Biome::FOREST; // centre
        let mut out = vec![Biome::NONE; 1];
        map_shore(MCVersion::V1_18, &parent, &mut out, 1, 1);
        assert_eq!(out[0], Biome::BEACH);
    }

    #[test]
    fn ocean_centre_stays_ocean() {
        let mut parent = vec![Biome::FOREST; 3 * 3];
        parent[3 + 1] = Biome::OCEAN;
        let mut out = vec![Biome::NONE; 1];
        map_shore(MCVersion::V1_18, &parent, &mut out, 1, 1);
        assert_eq!(out[0], Biome::OCEAN);
    }

    #[test]
    fn mountains_next_to_ocean_become_stone_shore_1_18() {
        let mut parent = vec![Biome::OCEAN; 3 * 3];
        parent[3 + 1] = Biome::MOUNTAINS;
        let mut out = vec![Biome::NONE; 1];
        map_shore(MCVersion::V1_18, &parent, &mut out, 1, 1);
        assert_eq!(out[0], Biome::STONE_SHORE);
    }

    #[test]
    fn snowy_taiga_next_to_ocean_becomes_snowy_beach() {
        let mut parent = vec![Biome::OCEAN; 3 * 3];
        parent[3 + 1] = Biome::SNOWY_TAIGA;
        let mut out = vec![Biome::NONE; 1];
        map_shore(MCVersion::V1_18, &parent, &mut out, 1, 1);
        assert_eq!(out[0], Biome::SNOWY_BEACH);
    }

    #[test]
    fn badlands_without_oceanic_neighbour_becomes_desert() {
        let mut parent = vec![Biome::FOREST; 3 * 3];
        parent[3 + 1] = Biome::BADLANDS;
        let mut out = vec![Biome::NONE; 1];
        map_shore(MCVersion::V1_18, &parent, &mut out, 1, 1);
        assert_eq!(out[0], Biome::DESERT);
    }

    #[test]
    fn mushroom_with_ocean_neighbour_becomes_shore() {
        let mut parent = vec![Biome::OCEAN; 3 * 3];
        parent[3 + 1] = Biome::MUSHROOM_FIELDS;
        let mut out = vec![Biome::NONE; 1];
        map_shore(MCVersion::V1_18, &parent, &mut out, 1, 1);
        assert_eq!(out[0], Biome::MUSHROOM_FIELD_SHORE);
    }

    #[test]
    fn mc_1_0_passes_anything_through() {
        let mut parent = vec![Biome::OCEAN; 3 * 3];
        parent[3 + 1] = Biome::FOREST;
        let mut out = vec![Biome::NONE; 1];
        map_shore(MCVersion::V1_0, &parent, &mut out, 1, 1);
        assert_eq!(out[0], Biome::FOREST);
    }

    #[test]
    fn mc_1_6_mountains_become_mountain_edge() {
        let mut parent = vec![Biome::OCEAN; 3 * 3];
        parent[3 + 1] = Biome::MOUNTAINS;
        let mut out = vec![Biome::NONE; 1];
        map_shore(MCVersion::V1_6, &parent, &mut out, 1, 1);
        // pre-1.7: mountains adjacent to non-mountains -> mountain_edge.
        assert_eq!(out[0], Biome::MOUNTAIN_EDGE);
    }
}
