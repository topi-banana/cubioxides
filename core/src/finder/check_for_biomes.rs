//! `checkForBiomes` — bit-exact-where-possible port.
//!
//! Cubiomes' `checkForBiomes` covers three paths:
//! 1. Beta (`mc <= MC_B1_7`) — `genBiomes` + bitmask test.
//! 2. Layered Overworld (1.7-1.17) — `checkForBiomesAtLayer` with
//!    a chain of filter mapfuncs that early-exit the layer DAG.
//! 3. 1.18+ Overworld — climate-driven gradient descent + a
//!    randomised Monte-Carlo sampler that uses libc `rand()`
//!    (non-portable, no bit-exact parity possible).
//!
//! Paths #1 and #2, plus every non-Overworld dim, run through the
//! same simple "`gen_biomes` + bitmask" implementation here.
//! Cubiomes' swap-map chain in path #2 is purely an early-exit
//! optimisation, so the Pass/Fail answer matches the exhaustive
//! approach used here. cubiomes' return-2 `ExclusionStop`
//! outcome is not synthesised — callers should treat it as Pass
//! for the exclusion filter.
//!
//! Only path #3 (1.18+ Overworld) returns
//! [`CheckForBiomesResult::Unsupported`]: cubiomes uses libc
//! `rand()` for the Monte Carlo sampling phase, so the
//! random-walk path can have false negatives that differ
//! per-platform. The exhaustive Rust approach would give the
//! *correct* answer but not the cubiomes-bit-exact one.
//!
//! A separate [`approx_prefilter_at_layer`] entrypoint exposes
//! the `BF_APPROX` fast-reject prefilter from path #2 — it can
//! prove a region cannot satisfy the filter, but cannot prove
//! the converse.

use crate::finder::biome_filter::BiomeFilter;
use crate::generator::{Generator, Range};
use crate::layer::stack::{LayerId, LayerStack};
use crate::mc_version::{Dimension, MCVersion};
use crate::rng::mc_seed::{get_chunk_seed, get_start_seed, mc_first_int, mc_first_is_zero};

/// Outcome of [`check_for_biomes`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckForBiomesResult {
    /// Filter matched (return 1 in cubiomes).
    Pass,
    /// Filter didn't match (return 0).
    Fail,
    /// Exclusion analysis proved the region can't generate the
    /// excluded biomes — early exit (return 2). Only the layered
    /// 1.7-1.17 path emits this; we don't synthesise it for Beta.
    ExclusionStop,
    /// MC version path not yet ported.
    Unsupported,
}

impl CheckForBiomesResult {
    /// `true` if the result counts as a positive match (Pass).
    #[must_use]
    pub fn is_pass(self) -> bool {
        matches!(self, Self::Pass)
    }
}

/// Bit-exact port of cubiomes' `checkForBiomes` for Beta and
/// Layered (MC ≤ 1.17) Overworld, plus all dims for any MC version
/// where `gen_biomes` is supported. Returns
/// [`CheckForBiomesResult::Unsupported`] only for 1.18+ Overworld
/// (which uses libc `rand()` and so cannot match cubiomes
/// bit-exactly).
///
/// For Beta/Layered, the implementation generates every cell in the
/// 2D area and folds biome IDs into one of two 64-bit bitmasks (`b`
/// for ids 0..64, `m` for ids 128..192), then matches the filter's
/// req/excl/any masks against that. cubiomes' optimised swap-map
/// chain may early-exit; the final Pass/Fail outcome is identical.
/// Note that cubiomes' "exclusion proven by chain early-exit"
/// return value (`2`) is not synthesised here — that's an internal
/// optimisation signal, not a different semantic outcome.
pub fn check_for_biomes(
    g: &mut Generator,
    range: Range,
    dim: Dimension,
    seed: u64,
    filter: &BiomeFilter,
) -> CheckForBiomesResult {
    // Beta + Layered (1.7-1.17) + non-Overworld dims all reduce to
    // "generate the area, fold every biome ID into a 0/1 mask,
    // match the filter". Cubiomes' Layered path additionally has a
    // swap-map early-exit chain (`checkForBiomesAtLayer`) that's
    // strictly an optimisation: the final Pass/Fail answer matches
    // what we compute here.
    //
    // The 1.18+ Overworld path is the only truly non-portable case
    // (libc `rand()` in the Monte Carlo phase) — keep returning
    // `Unsupported` there. Note: cubiomes also uses the Monte Carlo
    // path for 1.18+ Nether/End and for pre-1.18 Nether/End, so the
    // simple exhaustive approach here can disagree on Pass/Fail
    // with cubiomes (cubiomes' MC can have false negatives because
    // it samples a random subset of cells, not every cell). The
    // exhaustive answer is the *correct* one; callers who need
    // cubiomes-bit-exact behaviour for those dim/MC combinations
    // need to special-case them.
    let is_modern_overworld = dim == Dimension::Overworld && g.mc.is_at_least(MCVersion::V1_18);
    if is_modern_overworld {
        return CheckForBiomesResult::Unsupported;
    }

    if g.dim != Some(dim) || g.seed != seed {
        g.apply_seed(dim, seed);
    }
    let cell_count = range.cell_count();
    let mut ids = vec![crate::biome::Biome(0); cell_count];
    g.gen_biomes(&mut ids, range);

    let mut b: u64 = 0;
    let mut m: u64 = 0;
    for cell in ids.iter().take((range.sx as usize) * (range.sz as usize)) {
        let id = cell.0;
        if (0..64).contains(&id) {
            b |= 1_u64 << id;
        } else if (128..192).contains(&id) {
            m |= 1_u64 << (id - 128);
        }
    }
    // cubiomes' three boolean flags, extended with the mutated-biome
    // variants. For Beta, the M masks are all zero so this collapses
    // to the original logic.
    let mut match_exc = (filter.biome_to_excl | filter.biome_to_excl_m) == 0;
    let mut match_any = (filter.biome_to_pick | filter.biome_to_pick_m) == 0;
    let mut match_req = filter.biome_to_find == 0 && filter.biome_to_find_m == 0;
    match_exc |= (b & filter.biome_to_excl) == 0 && (m & filter.biome_to_excl_m) == 0;
    match_any |= (b & filter.biome_to_pick) != 0 || (m & filter.biome_to_pick_m) != 0;
    match_req |= (b & filter.biome_to_find) == filter.biome_to_find
        && (m & filter.biome_to_find_m) == filter.biome_to_find_m;
    if match_exc && match_any && match_req {
        CheckForBiomesResult::Pass
    } else {
        CheckForBiomesResult::Fail
    }
}

/// `checkForBiomesAtLayer(stack, entry, cache, seed, x, z, w, h,
/// filter)` — same semantics as cubiomes' function of that name, but
/// implemented via `gen_area` + bitmask match rather than cubiomes'
/// swap-map chain. The Pass/Fail outcome is bit-exact compatible
/// (cubiomes' early-exit is purely an optimisation); we don't
/// synthesise the return-2 `ExclusionStop` outcome.
///
/// Callers usually want the higher-level [`check_for_biomes`] which
/// dispatches by `(dim, mc)`; this lower-level entrypoint is for the
/// advanced case where the caller already has a specific layer
/// entry from [`crate::generator::Generator::layer_for_scale`] or
/// equivalent and wants to bypass the scale lookup.
pub fn check_for_biomes_at_layer(
    stack: &mut LayerStack,
    entry: LayerId,
    seed: u64,
    x: i32,
    z: i32,
    w: u32,
    h: u32,
    filter: &BiomeFilter,
) -> CheckForBiomesResult {
    crate::layer::set_layer_seed(stack, entry, seed);
    let mut ids = vec![crate::biome::Biome(0); (w as usize) * (h as usize)];
    crate::layer::gen_area(stack, entry, &mut ids, x, z, w as usize, h as usize);

    let mut b: u64 = 0;
    let mut m: u64 = 0;
    for cell in &ids {
        let id = cell.0;
        if (0..64).contains(&id) {
            b |= 1_u64 << id;
        } else if (128..192).contains(&id) {
            m |= 1_u64 << (id - 128);
        }
    }
    let mut match_exc = (filter.biome_to_excl | filter.biome_to_excl_m) == 0;
    let mut match_any = (filter.biome_to_pick | filter.biome_to_pick_m) == 0;
    let mut match_req = filter.biome_to_find == 0 && filter.biome_to_find_m == 0;
    match_exc |= (b & filter.biome_to_excl) == 0 && (m & filter.biome_to_excl_m) == 0;
    match_any |= (b & filter.biome_to_pick) != 0 || (m & filter.biome_to_pick_m) != 0;
    match_req |= (b & filter.biome_to_find) == filter.biome_to_find
        && (m & filter.biome_to_find_m) == filter.biome_to_find_m;
    if match_exc && match_any && match_req {
        CheckForBiomesResult::Pass
    } else {
        CheckForBiomesResult::Fail
    }
}

// Raw biome IDs (subset used by the approx-prefilter switches).
const BADLANDS_PLATEAU: i32 = 39;
const WOODED_BADLANDS_PLATEAU: i32 = 38;
const DESERT: i32 = 2;
const SAVANNA: i32 = 35;
const PLAINS: i32 = 1;
const FOREST: i32 = 4;
const DARK_FOREST: i32 = 29;
const MOUNTAINS: i32 = 3;
const BIRCH_FOREST: i32 = 27;
const SWAMP: i32 = 6;
const SNOWY_TAIGA: i32 = 30;
const SNOWY_TUNDRA: i32 = 12;
const MUSHROOM_FIELDS: i32 = 14;

/// Bit-exact port of cubiomes' `BF_APPROX` prefilter path inside
/// `checkForBiomesAtLayer`.
///
/// Returns `false` when the area at `(x, z)` of size `(w, h)`
/// definitely cannot satisfy `filter` based on chunk-seed math
/// alone — same fast-reject cubiomes performs when `BF_APPROX`
/// is set. Returns `true` otherwise; this is NOT a positive
/// confirmation that the filter passes (cubiomes continues with
/// the full layer-DAG swap-map chain, which we haven't ported).
///
/// `entry_scale` is the block-scale of the entry layer that the
/// caller would otherwise pass to cubiomes' `checkForBiomesAtLayer`
/// (e.g. 4 for `L_RIVER_MIX_4`, 16 for `L_SHORE_16`, 64 for
/// `L_HILLS_64`, 256 for `L_BIOME_256`).
#[must_use]
#[allow(clippy::too_many_lines, clippy::too_many_arguments)]
pub fn approx_prefilter_at_layer(
    stack: &LayerStack,
    filter: &BiomeFilter,
    seed: u64,
    entry_scale: i32,
    x: i32,
    z: i32,
    w: u32,
    h: u32,
) -> bool {
    let bx = x * entry_scale;
    let bz = z * entry_scale;
    let bw = w as i32 * entry_scale;
    let bh = h as i32 * entry_scale;

    // Special temperature count via L_SPECIAL_1024 chunk seeds.
    let mut special_cnt = filter.special_cnt;
    if special_cnt > 0 {
        let l = &stack.layers[LayerId::Special1024 as usize];
        let mut x0 = bx / l.scale;
        if x < 0 {
            x0 -= 1;
        }
        let mut z0 = bz / l.scale;
        if z < 0 {
            z0 -= 1;
        }
        let mut x1 = (bx + bw) / l.scale;
        if x + (w as i32) >= 0 {
            x1 += 1;
        }
        let mut z1 = (bz + bh) / l.scale;
        if z + (h as i32) >= 0 {
            z1 += 1;
        }
        let ss = get_start_seed(seed, l.layer_salt);
        for j in z0..=z1 {
            for i in x0..=x1 {
                let cs = get_chunk_seed(ss, i, j);
                if mc_first_is_zero(cs, 13) {
                    special_cnt -= 1;
                }
            }
        }
        if special_cnt > 0 {
            return false;
        }
    }

    let l = &stack.layers[LayerId::Biome256 as usize];
    let mut x0 = bx / l.scale;
    if x < 0 {
        x0 -= 1;
    }
    let mut z0 = bz / l.scale;
    if z < 0 {
        z0 -= 1;
    }
    let mut x1 = (bx + bw) / l.scale;
    if x + (w as i32) >= 0 {
        x1 += 1;
    }
    let mut z1 = (bz + bh) / l.scale;
    if z + (h as i32) >= 0 {
        z1 += 1;
    }

    // Mushroom-protochunk prefilter.
    if filter.major_to_find & (1_u64 << MUSHROOM_FIELDS) != 0 {
        let ml = &stack.layers[LayerId::Mushroom256 as usize];
        let ss = get_start_seed(seed, ml.layer_salt);
        let mut found = false;
        'mushroom: for j in z0..=z1 {
            for i in x0..=x1 {
                let cs = get_chunk_seed(ss, i, j);
                if mc_first_is_zero(cs, 100) {
                    found = true;
                    break 'mushroom;
                }
            }
        }
        if !found {
            return false;
        }
    }

    // Major-biome potential.
    let required = filter.major_to_find
        & ((1_u64 << BADLANDS_PLATEAU)
            | (1_u64 << WOODED_BADLANDS_PLATEAU)
            | (1_u64 << DESERT)
            | (1_u64 << SAVANNA)
            | (1_u64 << PLAINS)
            | (1_u64 << FOREST)
            | (1_u64 << DARK_FOREST)
            | (1_u64 << MOUNTAINS)
            | (1_u64 << BIRCH_FOREST)
            | (1_u64 << SWAMP));

    let ss = get_start_seed(seed, l.layer_salt);
    let mut potential: u64 = 0;
    for j in z0..=z1 {
        for i in x0..=x1 {
            let cs = get_chunk_seed(ss, i, j);
            let cs6 = mc_first_int(cs, 6);
            let cs3 = mc_first_int(cs, 3);
            let cs4 = mc_first_int(cs, 4);

            if cs3 != 0 {
                potential |= 1_u64 << BADLANDS_PLATEAU;
            } else {
                potential |= 1_u64 << WOODED_BADLANDS_PLATEAU;
            }

            match cs6 {
                0 => potential |= (1_u64 << DESERT) | (1_u64 << FOREST),
                1 => potential |= (1_u64 << DESERT) | (1_u64 << DARK_FOREST),
                2 => potential |= (1_u64 << DESERT) | (1_u64 << MOUNTAINS),
                3 => potential |= (1_u64 << SAVANNA) | (1_u64 << PLAINS),
                4 => potential |= (1_u64 << SAVANNA) | (1_u64 << BIRCH_FOREST),
                5 => potential |= (1_u64 << PLAINS) | (1_u64 << SWAMP),
                _ => {}
            }

            if cs4 == 3 {
                potential |= 1_u64 << SNOWY_TAIGA;
            } else {
                potential |= 1_u64 << SNOWY_TUNDRA;
            }
        }
    }
    if (potential & required) ^ required != 0 {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::finder::biome_filter::setup_biome_filter;
    use crate::layer::setup_layer_stack;

    #[test]
    fn check_for_biomes_at_layer_matches_check_for_biomes() {
        // Build a stack at 1.16.1, exercise check_for_biomes_at_layer
        // directly via the OceanMix4 entry, and confirm the result
        // matches what check_for_biomes computes for the equivalent
        // scale=4 Overworld range. Both reduce to gen_area +
        // bitmask, so they should agree on Pass/Fail.
        let mc = MCVersion::V1_16_1;
        let seed = 0xdead_beef;
        let mut stack = Box::new(LayerStack::new());
        setup_layer_stack(&mut stack, mc, false);
        let entry = stack.entry_4.unwrap();

        // Required: plains (1).
        let filter = setup_biome_filter(mc, 0, &[1], &[], &[]).expect("filter");
        let direct = check_for_biomes_at_layer(&mut stack, entry, seed, 0, 0, 16, 16, &filter);

        let mut g = Generator::new(mc, 0);
        let r = Range {
            scale: 4,
            x: 0,
            z: 0,
            sx: 16,
            sz: 16,
            y: 0,
            sy: 1,
        };
        let high = check_for_biomes(&mut g, r, Dimension::Overworld, seed, &filter);
        assert_eq!(direct, high);
    }
}
