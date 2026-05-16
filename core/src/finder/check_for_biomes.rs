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
//! [`CheckForBiomesResult::Unsupported`].

use crate::finder::biome_filter::BiomeFilter;
use crate::generator::{Generator, Range};
use crate::mc_version::{Dimension, MCVersion};

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
