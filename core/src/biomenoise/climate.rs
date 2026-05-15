//! 1.18+ climate-to-biome decision-tree lookup.
//!
//! Bit-exact port of cubiomes' `climateToBiome` + the internal
//! `get_np_dist` / `get_resulting_node` helpers. The 6-axis climate
//! point (temperature, humidity, continentalness, erosion, depth,
//! weirdness) is walked against a packed binary decision tree
//! sourced from `tables/btree*.h` and selected by MC version.
//!
//! Each `BiomeTree::nodes[i]` is a 64-bit word:
//!
//! ```text
//!   bits 56..63 = 0xff for a leaf, else a child index high byte
//!   bits 48..55 = leaf biome id when high byte == 0xff, else child low byte
//!   bits 0..47  = six 8-bit indices into `param`, one per climate axis
//! ```
//!
//! The decision-tree walk keeps a running squared-distance metric
//! (`get_np_dist`) and recursively explores any sibling whose
//! current-axis distance is below the best so far.

#![allow(clippy::many_single_char_names, clippy::too_many_arguments)]

use crate::mc_version::MCVersion;

/// Static description of a single MC-version decision tree.
#[derive(Debug, Clone, Copy)]
pub struct BiomeTree {
    /// Per-depth child-index strides. The walk stops at the depth
    /// whose entry is zero.
    pub steps: &'static [u32],
    /// Flat array of `(min, max)` climate bounds in units of 1/10000. Pair
    /// `i` lives at `param[2 * i]` and `param[2 * i + 1]`.
    pub param: &'static [i32],
    /// Packed decision-tree nodes. See module docs for the layout.
    pub nodes: &'static [u64],
    /// Tree fan-out (cubiomes' `btreeN_order`).
    pub order: u32,
}

impl BiomeTree {
    /// Length of [`Self::nodes`] — cubiomes' `bt->len`.
    #[inline]
    #[must_use]
    pub const fn len(&self) -> u32 {
        self.nodes.len() as u32
    }

    /// Returns `true` if the tree carries no nodes (only true for
    /// degenerate / mock tables).
    #[inline]
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}

/// Pick the decision tree cubiomes uses for the given MC version.
/// `climateToBiome` selects in the same order: 1.21 WD → 1.20.6 →
/// 1.19.4 → 1.19.2 → fall back to 1.18.
#[must_use]
pub fn tree_for_version(mc: MCVersion) -> &'static BiomeTree {
    use crate::tables;
    if mc.is_at_least(MCVersion::V1_21) {
        &tables::BTREE_21WD
    } else if mc.is_at_least(MCVersion::V1_20) {
        &tables::BTREE_20
    } else if mc.is_at_least(MCVersion::V1_19) {
        &tables::BTREE_19
    } else if mc.is_at_least(MCVersion::V1_19_2) {
        &tables::BTREE_192
    } else {
        &tables::BTREE_18
    }
}

/// `get_np_dist` — cubiomes' inner-most distance kernel. For each of
/// the six climate axes, accumulates the squared "distance outside
/// the bound" of `np[i]` against `param[2 * idx]..=param[2 * idx +
/// 1]`. Uses `i64` saturation semantics to match the C
/// implementation's `(int64_t)x > 0` checks.
#[must_use]
pub fn get_np_dist(np: &[u64; 6], bt: &BiomeTree, idx: usize) -> u64 {
    let node = bt.nodes[idx];
    let mut ds: u64 = 0;
    for (i, &n) in np.iter().enumerate() {
        let param_idx = ((node >> (8 * i)) & 0xFF) as usize;
        // n - param[2*idx + 1]  ==  np[i] - upper bound
        let a = n.wrapping_sub(bt.param[2 * param_idx + 1] as i64 as u64);
        // param[2*idx + 0] - n  ==  lower bound - np[i]
        let b = (bt.param[2 * param_idx] as i64 as u64).wrapping_sub(n);
        let d = if (a as i64) > 0 {
            a
        } else if (b as i64) > 0 {
            b
        } else {
            0
        };
        ds = ds.wrapping_add(d.wrapping_mul(d));
    }
    ds
}

/// `get_resulting_node` — recursive tree descent. Mirrors cubiomes
/// verbatim, including the "skip depths whose stride overshoots
/// `len`" loop and the running best-distance pruning.
#[must_use]
pub fn get_resulting_node(
    np: &[u64; 6],
    bt: &BiomeTree,
    idx: usize,
    alt: usize,
    ds: u64,
    depth: usize,
) -> usize {
    if bt.steps[depth] == 0 {
        return idx;
    }

    let len = bt.len() as usize;
    let mut step = bt.steps[depth] as usize;
    let mut depth = depth;
    while idx + step >= len {
        depth += 1;
        step = bt.steps[depth] as usize;
    }
    depth += 1;

    let node = bt.nodes[idx];
    let mut inner = ((node >> 48) & 0xFFFF) as usize;
    let mut leaf = alt;
    let mut ds = ds;

    for _ in 0..bt.order {
        let ds_inner = get_np_dist(np, bt, inner);
        if ds_inner < ds {
            let leaf2 = get_resulting_node(np, bt, inner, leaf, ds, depth);
            let ds_leaf2 = if inner == leaf2 {
                ds_inner
            } else {
                get_np_dist(np, bt, leaf2)
            };
            if ds_leaf2 < ds {
                ds = ds_leaf2;
                leaf = leaf2;
            }
        }
        inner += step;
        if inner >= len {
            break;
        }
    }
    leaf
}

/// `climateToBiome(mc, np, dat)` — pick a biome id from a 6-tuple
/// climate point. `dat`, when present, carries the previous cell's
/// chosen leaf index for cubiomes' chunk-section short-circuit
/// optimisation (passed in / out as a hint).
#[must_use]
pub fn climate_to_biome(mc: MCVersion, np: &[u64; 6], dat: Option<&mut u64>) -> i32 {
    let bt = tree_for_version(mc);
    let idx = if let Some(d) = dat {
        let alt = *d as usize;
        let ds = get_np_dist(np, bt, alt);
        let result = get_resulting_node(np, bt, 0, alt, ds, 0);
        *d = result as u64;
        result
    } else {
        get_resulting_node(np, bt, 0, 0, u64::MAX, 0)
    };
    ((bt.nodes[idx] >> 48) & 0xFF) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_for_version_returns_btree18_for_1_18() {
        let bt = tree_for_version(MCVersion::V1_18);
        assert_eq!(bt.order, 10);
        assert!(bt.len() > 1000);
    }

    #[test]
    fn climate_to_biome_zero_climate_is_deterministic() {
        let np = [0u64; 6];
        let a = climate_to_biome(MCVersion::V1_18, &np, None);
        let b = climate_to_biome(MCVersion::V1_18, &np, None);
        assert_eq!(a, b);
        assert!(a >= 0);
    }

    #[test]
    fn dat_passthrough_round_trips() {
        let np = [1000u64; 6];
        let mut dat = 0u64;
        let id1 = climate_to_biome(MCVersion::V1_18, &np, Some(&mut dat));
        let dat_after = dat;
        let id2 = climate_to_biome(MCVersion::V1_18, &np, Some(&mut dat));
        assert_eq!(id1, id2);
        assert_eq!(dat, dat_after);
    }
}
