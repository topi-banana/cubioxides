//! `checkForBiomes` — partial port (Beta 1.7 only for now).
//!
//! Cubiomes' `checkForBiomes` covers three radically different paths:
//! 1. Beta (`mc <= MC_B1_7`) — `genBiomes` + bitmask test.
//! 2. Layered Overworld (1.7-1.17) — `checkForBiomesAtLayer` with
//!    a chain of filter mapfuncs that early-exit the layer DAG.
//! 3. 1.18+ — climate-driven gradient descent + a randomised
//!    Monte-Carlo sampler that uses libc `rand()` (non-portable,
//!    no bit-exact parity possible).
//!
//! Only path #1 is currently ported. Paths #2 and #3 return
//! [`CheckForBiomesResult::Unsupported`]. A separate
//! [`approx_prefilter_at_layer`] entrypoint exposes the
//! `BF_APPROX` fast-reject prefilter from path #2 — it can prove a
//! region cannot satisfy the filter, but cannot prove the
//! converse.

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

/// Bit-exact port of cubiomes' Beta-era `checkForBiomes`. Other
/// MC versions return [`CheckForBiomesResult::Unsupported`] —
/// callers should special-case.
///
/// The `cache` arg is allowed to be `None`; we allocate a fresh
/// `Vec<Biome>` in that case (matching cubiomes' `allocCache`).
pub fn check_for_biomes(
    g: &mut Generator,
    range: Range,
    dim: Dimension,
    seed: u64,
    filter: &BiomeFilter,
) -> CheckForBiomesResult {
    if !g.mc.is_at_least(MCVersion::B1_8) {
        // Re-seed if dim or seed changed.
        if g.dim != Some(dim) || g.seed != seed {
            g.apply_seed(dim, seed);
        }
        let cell_count = range.cell_count();
        let mut ids = vec![crate::biome::Biome(0); cell_count];
        g.gen_biomes(&mut ids, range);

        let mut b: u64 = 0;
        for cell in ids.iter().take((range.sx as usize) * (range.sz as usize)) {
            let id = cell.0;
            if (0..64).contains(&id) {
                b |= 1_u64 << id;
            }
        }
        // Re-derive cubiomes' three boolean flags.
        let mut match_exc = filter.biome_to_excl == 0;
        let mut match_any = filter.biome_to_pick == 0;
        let mut match_req = filter.biome_to_find == 0;
        match_exc |= (b & filter.biome_to_excl) == 0;
        match_any |= (b & filter.biome_to_pick) != 0;
        match_req |= (b & filter.biome_to_find) == filter.biome_to_find;
        if match_exc && match_any && match_req {
            return CheckForBiomesResult::Pass;
        }
        return CheckForBiomesResult::Fail;
    }
    CheckForBiomesResult::Unsupported
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
