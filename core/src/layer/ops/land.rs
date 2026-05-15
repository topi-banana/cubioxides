//! `mapLand` (a.k.a. `mapAddIsland`) — adds and trims islands.
//!
//! Bit-exact port of `mapLand` in `cubiomes/layers.c`. The function
//! walks a `(w, h)` window and consults the four diagonal neighbours of
//! each cell in the `(w+2) × (h+2)` parent rectangle:
//!
//! ```text
//!   v00 . v20
//!    .  v11 .
//!   v02 . v22
//! ```
//!
//! Ocean cells with at least one non-ocean diagonal neighbour may
//! "grow" into one of the neighbouring biomes. Non-ocean cells with at
//! least one ocean diagonal neighbour may shrink back into ocean. The
//! exact picks come from a chain of `mcFirstIsZero` calls seeded from
//! the chunk seed, replicating cubiomes' Java RNG sequence exactly.

#![allow(clippy::many_single_char_names)] // mirrors cubiomes/layers.c

use crate::biome::Biome;
use crate::rng::{get_chunk_seed, mc_first_is_zero, mc_step_seed};

const OCEAN: i32 = Biome::OCEAN.id();
const FOREST: i32 = Biome::FOREST.id();

/// `mapLand` — port of cubiomes' inline island-add / island-trim layer.
///
/// `parent` must contain `(w + 2) * (h + 2)` cells covering the
/// rectangle whose top-left corner is `(x - 1, z - 1)`. `out` receives
/// the `w * h` window starting at `(x, z)`.
#[allow(clippy::too_many_arguments)]
pub fn map_land(
    start_salt: u64,
    start_seed: u64,
    parent: &[Biome],
    out: &mut [Biome],
    x: i32,
    z: i32,
    w: usize,
    h: usize,
) {
    let p_w = w + 2;
    let p_h = h + 2;
    assert!(
        parent.len() >= p_w * p_h,
        "map_land: parent slice too small"
    );
    assert!(out.len() >= w * h, "map_land: output slice too small");

    for j in 0..h {
        // Rolling state across the inner loop: v00 is the upper-left
        // diagonal of the cell currently being processed, vt0 / vt2 hold
        // the next-column-over values so the next iteration can promote
        // them without re-reading the parent row.
        let mut v00 = parent[j * p_w].id();
        let mut vt0 = parent[1 + j * p_w].id();
        let mut v02 = parent[(j + 2) * p_w].id();
        let mut vt2 = parent[1 + (j + 2) * p_w].id();

        for i in 0..w {
            let v11 = parent[(i + 1) + (j + 1) * p_w].id();
            let v20 = parent[(i + 2) + j * p_w].id();
            let v22 = parent[(i + 2) + (j + 2) * p_w].id();
            let mut v = v11;

            if v11 == OCEAN {
                if v00 != OCEAN || v20 != OCEAN || v02 != OCEAN || v22 != OCEAN {
                    let mut cs = get_chunk_seed(start_seed, i as i32 + x, j as i32 + z);
                    let mut inc: i32 = 0;
                    v = 1;

                    if v00 != OCEAN {
                        inc += 1;
                        v = v00;
                        cs = mc_step_seed(cs, start_salt);
                    }
                    if v20 != OCEAN {
                        inc += 1;
                        if inc == 1 || mc_first_is_zero(cs, 2) {
                            v = v20;
                        }
                        cs = mc_step_seed(cs, start_salt);
                    }
                    if v02 != OCEAN {
                        inc += 1;
                        match inc {
                            1 => v = v02,
                            2 => {
                                if mc_first_is_zero(cs, 2) {
                                    v = v02;
                                }
                            }
                            _ => {
                                if mc_first_is_zero(cs, 3) {
                                    v = v02;
                                }
                            }
                        }
                        cs = mc_step_seed(cs, start_salt);
                    }
                    if v22 != OCEAN {
                        inc += 1;
                        match inc {
                            1 => v = v22,
                            2 => {
                                if mc_first_is_zero(cs, 2) {
                                    v = v22;
                                }
                            }
                            3 => {
                                if mc_first_is_zero(cs, 3) {
                                    v = v22;
                                }
                            }
                            _ => {
                                if mc_first_is_zero(cs, 4) {
                                    v = v22;
                                }
                            }
                        }
                        cs = mc_step_seed(cs, start_salt);
                    }

                    if v != FOREST && !mc_first_is_zero(cs, 3) {
                        v = OCEAN;
                    }
                }
            } else if v11 == FOREST {
                // Pass-through: forest is preserved verbatim.
            } else {
                // Non-ocean, non-forest with at least one ocean neighbour
                // can shrink back into ocean with probability 1/5.
                if v00 == OCEAN || v20 == OCEAN || v02 == OCEAN || v22 == OCEAN {
                    let cs = get_chunk_seed(start_seed, i as i32 + x, j as i32 + z);
                    if mc_first_is_zero(cs, 5) {
                        v = OCEAN;
                    }
                }
            }

            out[i + j * w] = Biome(v);

            // Slide the rolling buffer one column to the right.
            v00 = vt0;
            vt0 = v20;
            v02 = vt2;
            vt2 = v22;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_ocean_parent(w: usize, h: usize) -> Vec<Biome> {
        vec![Biome::OCEAN; (w + 2) * (h + 2)]
    }

    fn all_forest_parent(w: usize, h: usize) -> Vec<Biome> {
        vec![Biome::FOREST; (w + 2) * (h + 2)]
    }

    #[test]
    fn pure_ocean_input_stays_ocean() {
        let parent = all_ocean_parent(8, 8);
        let mut out = vec![Biome::NONE; 8 * 8];
        map_land(0, 1, &parent, &mut out, 0, 0, 8, 8);
        for &cell in &out {
            assert_eq!(cell, Biome::OCEAN);
        }
    }

    #[test]
    fn pure_forest_input_stays_forest() {
        let parent = all_forest_parent(8, 8);
        let mut out = vec![Biome::NONE; 8 * 8];
        map_land(0, 1, &parent, &mut out, 0, 0, 8, 8);
        for &cell in &out {
            assert_eq!(cell, Biome::FOREST);
        }
    }

    #[test]
    fn output_window_size_matches_w_times_h() {
        let parent = all_forest_parent(13, 7);
        let mut out = vec![Biome::NONE; 13 * 7];
        map_land(123, 456, &parent, &mut out, -5, -3, 13, 7);
        for cell in &out {
            assert_ne!(*cell, Biome::NONE);
        }
    }

    #[test]
    #[should_panic(expected = "parent slice too small")]
    fn panics_on_undersized_parent() {
        let parent = vec![Biome::OCEAN; 4];
        let mut out = vec![Biome::NONE; 4 * 4];
        map_land(0, 0, &parent, &mut out, 0, 0, 4, 4);
    }
}
