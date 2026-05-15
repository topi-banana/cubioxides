//! `mapDeepOcean` — promote shallow ocean surrounded by ocean to deep.
//!
//! Bit-exact port of cubiomes' `mapDeepOcean`. A shallow-ocean cell
//! becomes its deep counterpart when at least four of its cardinal
//! neighbours are also shallow ocean.

use crate::biome::Biome;

/// `mapDeepOcean` — read a `(w+2, h+2)` parent and emit a `(w, h)` window.
pub fn map_deep_ocean(parent: &[Biome], out: &mut [Biome], w: usize, h: usize) {
    let p_w = w + 2;
    assert!(
        parent.len() >= p_w * (h + 2),
        "map_deep_ocean: parent slice too small"
    );
    assert!(out.len() >= w * h, "map_deep_ocean: output slice too small");

    for j in 0..h {
        for i in 0..w {
            let mut v11 = parent[(i + 1) + (j + 1) * p_w].id();

            if Biome::is_shallow_ocean_id(v11) {
                let mut oceans = 0;
                if Biome::is_shallow_ocean_id(parent[(i + 1) + j * p_w].id()) {
                    oceans += 1;
                }
                if Biome::is_shallow_ocean_id(parent[(i + 2) + (j + 1) * p_w].id()) {
                    oceans += 1;
                }
                if Biome::is_shallow_ocean_id(parent[i + (j + 1) * p_w].id()) {
                    oceans += 1;
                }
                if Biome::is_shallow_ocean_id(parent[(i + 1) + (j + 2) * p_w].id()) {
                    oceans += 1;
                }

                if oceans >= 4 {
                    // The `OCEAN` arm and the wildcard arm intentionally
                    // share a value: cubiomes' switch lists `case ocean:
                    // v = deep_ocean; default: v = deep_ocean;` and the
                    // explicit case keeps the per-variant mapping
                    // legible.
                    #[allow(clippy::match_same_arms)]
                    {
                        v11 = match Biome(v11) {
                            Biome::WARM_OCEAN => Biome::DEEP_WARM_OCEAN.id(),
                            Biome::LUKEWARM_OCEAN => Biome::DEEP_LUKEWARM_OCEAN.id(),
                            Biome::OCEAN => Biome::DEEP_OCEAN.id(),
                            Biome::COLD_OCEAN => Biome::DEEP_COLD_OCEAN.id(),
                            Biome::FROZEN_OCEAN => Biome::DEEP_FROZEN_OCEAN.id(),
                            _ => Biome::DEEP_OCEAN.id(),
                        };
                    }
                }
            }

            out[i + j * w] = Biome(v11);
        }
    }
}
