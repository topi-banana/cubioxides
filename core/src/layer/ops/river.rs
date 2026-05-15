//! `mapRiver` — river generation layer.
//!
//! Bit-exact port of cubiomes' `mapRiver`. Reads a `(w+2, h+2)` parent
//! rectangle and emits a `(w, h)` window. For 1.7+ all five cardinal
//! cells are first run through `reduceID` (`id >= 2 ? 2 + (id & 1) :
//! id`); the centre becomes `-1` (none) when its four cardinals
//! reduce to the same value, otherwise it becomes `river`. Pre-1.7
//! the layer short-circuits: cells whose centre is `0` are forced to
//! `river` without inspecting the neighbours.

#![allow(clippy::many_single_char_names)]

use crate::biome::Biome;
use crate::mc_version::MCVersion;

#[inline]
const fn reduce_id(id: i32) -> i32 {
    if id >= 2 { 2 + (id & 1) } else { id }
}

const RIVER: i32 = Biome::RIVER.id();

/// `mapRiver` — parent `(w+2, h+2)`, output `(w, h)`.
pub fn map_river(mc: MCVersion, parent: &[Biome], out: &mut [Biome], w: usize, h: usize) {
    let p_w = w + 2;
    assert!(
        parent.len() >= p_w * (h + 2),
        "map_river: parent slice too small"
    );
    assert!(out.len() >= w * h, "map_river: output slice too small");

    let mc_ge_1_7 = mc.is_at_least(MCVersion::V1_7);

    for j in 0..h {
        for i in 0..w {
            let mut v01 = parent[i + (j + 1) * p_w].id();
            let mut v11 = parent[(i + 1) + (j + 1) * p_w].id();
            let mut v21 = parent[(i + 2) + (j + 1) * p_w].id();
            let mut v10 = parent[(i + 1) + j * p_w].id();
            let mut v12 = parent[(i + 1) + (j + 2) * p_w].id();

            if mc_ge_1_7 {
                v01 = reduce_id(v01);
                v11 = reduce_id(v11);
                v21 = reduce_id(v21);
                v10 = reduce_id(v10);
                v12 = reduce_id(v12);
            } else if v11 == 0 {
                out[i + j * w] = Biome(RIVER);
                continue;
            }

            let result = if v11 == v01 && v11 == v10 && v11 == v12 && v11 == v21 {
                Biome::NONE.id()
            } else {
                RIVER
            };
            out[i + j * w] = Biome(result);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_parent_yields_none_post_1_7() {
        // After reduceID, all values collapse to 2 + (id & 1), so
        // uniform input yields uniform reduced cardinals -> NONE.
        let parent = vec![Biome::FOREST; 6 * 6];
        let mut out = vec![Biome::FOREST; 4 * 4];
        map_river(MCVersion::V1_18, &parent, &mut out, 4, 4);
        for cell in &out {
            assert_eq!(*cell, Biome::NONE);
        }
    }

    #[test]
    fn pre_1_7_zero_centre_emits_river() {
        let parent = vec![Biome::OCEAN; 6 * 6];
        let mut out = vec![Biome::NONE; 4 * 4];
        map_river(MCVersion::V1_6, &parent, &mut out, 4, 4);
        for cell in &out {
            assert_eq!(*cell, Biome::RIVER);
        }
    }
}
