//! `checkForTemps` — pre-1.18 temperature-category histogram check.
//!
//! Bit-exact port of cubiomes' `checkForTemps` from `finders.c`. Used
//! by structure finders that pre-filter on the `L_SPECIAL_1024` layer
//! before running a more expensive biome check.

use crate::biome::Biome;
use crate::layer::cache::get_min_layer_cache_size;
use crate::layer::dispatch::gen_area;
use crate::layer::stack::{LayerId, LayerStack, set_layer_seed};
use crate::rng::mc_seed::{get_chunk_seed, get_layer_salt, get_start_seed, mc_first_is_zero};

/// Indexes into the `tc[9]` array. Mirrors cubiomes'
/// `BiomeTempCategory` arithmetic: indices 0–4 are the base
/// categories, 6–8 are the `Warm/Lush/Cold + Special` slots.
/// Index 5 (`Special + Oceanic`) and 9 (`Special + Freezing`) are
/// not addressable here because cubiomes uses a 9-wide array
/// indexed by `(temp & 0xff) + Special-flag`.
pub const TC_LEN: usize = 9;

/// Returns `true` (1) if the temperature histogram of the
/// `L_SPECIAL_1024` area satisfies `tc`: every slot must reach its
/// minimum count, and negative counts mark slots that must remain
/// at zero.
///
/// `tc` is indexed by `BiomeTempCategory` value (see [`TC_LEN`]).
///
/// # Example
///
/// ```
/// use cubioxides::MCVersion;
/// use cubioxides::finder::check_for_temps;
/// use cubioxides::layer::stack::{LayerStack, setup_layer_stack};
///
/// // Cheap pre-1.18 pre-filter on the L_SPECIAL_1024 area: every
/// // non-zero tc slot needs at least that many cells in the
/// // (x, z, w, h) window. A `tc` of all zeros trivially passes.
/// let mut stack = LayerStack::new();
/// setup_layer_stack(&mut stack, MCVersion::V1_16_1, false);
/// let tc = [0i32; cubioxides::finder::check_for_temps::TC_LEN];
/// let ok = check_for_temps(&mut stack, 0xdead_beef, 0, 0, 4, 4, &tc);
/// assert!(ok);
/// ```
#[must_use]
pub fn check_for_temps(
    stack: &mut LayerStack,
    seed: u64,
    x: i32,
    z: i32,
    w: i32,
    h: i32,
    tc: &[i32; TC_LEN],
) -> bool {
    // L_SPECIAL_1024 uses salt seed 3.
    let ls = get_layer_salt(3);
    let ss = get_start_seed(seed, ls);

    // Special-category Warm/Lush/Cold counts: pre-check via chunk seed,
    // because the L_SPECIAL_1024 area can short-circuit if there
    // aren't enough Special-flagged chunks in the bounding region.
    let mut scnt: i32 = 0;
    // tc[Special+Warm] = tc[6], etc.
    if tc[6] > 0 {
        scnt += tc[6];
    }
    if tc[7] > 0 {
        scnt += tc[7];
    }
    if tc[8] > 0 {
        scnt += tc[8];
    }
    if scnt > 0 {
        for j in 0..h {
            for i in 0..w {
                if mc_first_is_zero(get_chunk_seed(ss, x + i, z + j), 13) {
                    scnt -= 1;
                }
            }
        }
        if scnt > 0 {
            return false;
        }
    }

    // Now generate the full area and count actual temperatures.
    set_layer_seed(stack, LayerId::Special1024, seed);
    let cache_size = get_min_layer_cache_size(stack, LayerId::Special1024, w, h);
    let mut area: Vec<Biome> = vec![Biome(0); cache_size];
    gen_area(
        stack,
        LayerId::Special1024,
        &mut area,
        x,
        z,
        w as usize,
        h as usize,
    );

    let mut ccnt = [0_i32; TC_LEN];
    for cell in area.iter().take((w as usize) * (h as usize)) {
        let id = cell.0;
        let t = id & 0xff;
        let bucket = if id != t && t != 4
        /* Freezing */
        {
            (t + 5) /* Special offset */ as usize
        } else {
            t as usize
        };
        if bucket < TC_LEN {
            ccnt[bucket] += 1;
        }
    }
    for i in 0..TC_LEN {
        if ccnt[i] < tc[i] || (ccnt[i] != 0 && tc[i] < 0) {
            return false;
        }
    }
    true
}
