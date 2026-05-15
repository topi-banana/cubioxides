//! `mapCool` and `mapHeat` — temperature-category smoothing layers.
//!
//! Bit-exact ports of cubiomes' `mapCool` (Warm cells adjacent to
//! Cold/Freezing collapse to Lush) and `mapHeat` (Freezing cells
//! adjacent to Warm/Lush collapse to Cold).

use crate::biome::Biome;

const WARM: i32 = 1;
const LUSH: i32 = 2;
const COLD: i32 = 3;
const FREEZING: i32 = 4;

#[inline]
fn is_any4(target: i32, a: i32, b: i32, c: i32, d: i32) -> bool {
    target == a || target == b || target == c || target == d
}

/// `mapCool` — Warm cells with any Cold/Freezing cardinal neighbour
/// collapse to Lush.
pub fn map_cool(parent: &[Biome], out: &mut [Biome], w: usize, h: usize) {
    let p_w = w + 2;
    assert!(
        parent.len() >= p_w * (h + 2),
        "map_cool: parent slice too small"
    );
    assert!(out.len() >= w * h, "map_cool: output slice too small");

    for j in 0..h {
        for i in 0..w {
            let mut v11 = parent[(i + 1) + (j + 1) * p_w].id();

            if v11 == WARM {
                let v10 = parent[(i + 1) + j * p_w].id();
                let v21 = parent[(i + 2) + (j + 1) * p_w].id();
                let v01 = parent[i + (j + 1) * p_w].id();
                let v12 = parent[(i + 1) + (j + 2) * p_w].id();
                if is_any4(COLD, v10, v21, v01, v12) || is_any4(FREEZING, v10, v21, v01, v12) {
                    v11 = LUSH;
                }
            }

            out[i + j * w] = Biome(v11);
        }
    }
}

/// `mapHeat` — Freezing cells with any Warm/Lush cardinal neighbour
/// collapse to Cold.
pub fn map_heat(parent: &[Biome], out: &mut [Biome], w: usize, h: usize) {
    let p_w = w + 2;
    assert!(
        parent.len() >= p_w * (h + 2),
        "map_heat: parent slice too small"
    );
    assert!(out.len() >= w * h, "map_heat: output slice too small");

    for j in 0..h {
        for i in 0..w {
            let mut v11 = parent[(i + 1) + (j + 1) * p_w].id();

            if v11 == FREEZING {
                let v10 = parent[(i + 1) + j * p_w].id();
                let v21 = parent[(i + 2) + (j + 1) * p_w].id();
                let v01 = parent[i + (j + 1) * p_w].id();
                let v12 = parent[(i + 1) + (j + 2) * p_w].id();
                if is_any4(WARM, v10, v21, v01, v12) || is_any4(LUSH, v10, v21, v01, v12) {
                    v11 = COLD;
                }
            }

            out[i + j * w] = Biome(v11);
        }
    }
}
