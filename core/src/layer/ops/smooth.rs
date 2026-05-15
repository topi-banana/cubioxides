//! `mapSmooth` — river-smoothing layer.
//!
//! Bit-exact port of cubiomes' `mapSmooth`. Reads a `(w+2, h+2)`
//! parent and emits a `(w, h)` window. A centre cell that disagrees
//! with its top / left neighbour is rewritten: if both diagonals
//! match (v01 == v21 and v10 == v12) the chunk seed's bit 24 picks
//! between left or top; otherwise the matching cardinal pair wins.

#![allow(clippy::many_single_char_names)]

use crate::biome::Biome;
use crate::rng::get_chunk_seed;

/// `mapSmooth` — parent `(w+2, h+2)`, output `(w, h)`.
#[allow(clippy::too_many_arguments)]
pub fn map_smooth(
    start_seed: u64,
    parent: &[Biome],
    out: &mut [Biome],
    x: i32,
    z: i32,
    w: usize,
    h: usize,
) {
    let p_w = w + 2;
    assert!(
        parent.len() >= p_w * (h + 2),
        "map_smooth: parent slice too small"
    );
    assert!(out.len() >= w * h, "map_smooth: output slice too small");

    for j in 0..h {
        for i in 0..w {
            let mut v11 = parent[(i + 1) + (j + 1) * p_w].id();
            let v01 = parent[i + (j + 1) * p_w].id();
            let v10 = parent[(i + 1) + j * p_w].id();

            if v11 != v01 || v11 != v10 {
                let v21 = parent[(i + 2) + (j + 1) * p_w].id();
                let v12 = parent[(i + 1) + (j + 2) * p_w].id();
                if v01 == v21 && v10 == v12 {
                    let cs = get_chunk_seed(start_seed, i as i32 + x, j as i32 + z);
                    v11 = if (cs & (1u64 << 24)) != 0 { v10 } else { v01 };
                } else {
                    if v01 == v21 {
                        v11 = v01;
                    }
                    if v10 == v12 {
                        v11 = v10;
                    }
                }
            }

            out[i + j * w] = Biome(v11);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_parent_is_idempotent() {
        let parent = vec![Biome::FOREST; 6 * 6];
        let mut out = vec![Biome::NONE; 4 * 4];
        map_smooth(123, &parent, &mut out, 0, 0, 4, 4);
        for cell in &out {
            assert_eq!(*cell, Biome::FOREST);
        }
    }

    #[test]
    fn isolated_centre_collapses_to_majority() {
        // 4x4 parent of OCEAN with a single FOREST cell at (1, 1).
        // For map_smooth's 1x1 inspection window we set up a 3x3
        // parent so the only output cell (i=0, j=0) reads:
        //   v00..v22 around (i+1=1, j+1=1) = the FOREST cell. The
        //   four cardinals v10=parent[1,0], v01=parent[0,1],
        //   v21=parent[2,1], v12=parent[1,2] are all OCEAN, so the
        //   centre collapses to OCEAN.
        let mut parent = vec![Biome::OCEAN; 3 * 3];
        parent[3 + 1] = Biome::FOREST; // (i=1, j=1)
        let mut out = vec![Biome::NONE; 1];
        map_smooth(0, &parent, &mut out, 0, 0, 1, 1);
        assert_eq!(out[0], Biome::OCEAN);
    }
}
