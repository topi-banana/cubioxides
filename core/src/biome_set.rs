//! Two-bitfield biome ID set. Bit-exact port of cubiomes'
//! `idSetAdd` / `idSetTest` static-inline helpers — partitioning
//! biome IDs into two 64-bit masks `m_low` and `m_mut`:
//!
//! - `[0, 64)` → bit `id` of `m_low` (the unmodified biomes)
//! - `[128, 192)` → bit `id - 128` of `m_mut` (the "mutated" variants)
//!
//! IDs outside those two ranges (`[64, 128)` and `[192, ∞)`) are
//! ignored.

#![allow(clippy::missing_panics_doc)]

/// Pair of 64-bit masks indexed by biome ID. Add via [`Self::add`],
/// query via [`Self::contains`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BiomeSet {
    /// Bits for biome IDs in `[0, 64)`.
    pub m_low: u64,
    /// Bits for biome IDs in `[128, 192)` (offset by 128).
    pub m_mut: u64,
}

impl BiomeSet {
    /// Empty set.
    #[must_use]
    pub const fn new() -> Self {
        Self { m_low: 0, m_mut: 0 }
    }

    /// `idSetAdd(mL, mM, id)` — set the bit for biome `id`. IDs
    /// outside `[0, 64)` and `[128, 192)` are silently ignored.
    #[inline]
    pub fn add(&mut self, id: i32) {
        match id & !0x3f {
            0 => self.m_low |= 1u64 << id,
            128 => self.m_mut |= 1u64 << (id - 128),
            _ => {}
        }
    }

    /// `idSetTest(mL, mM, id)` — return `true` iff the bit for
    /// biome `id` is set. IDs outside the supported ranges return
    /// `false`.
    #[inline]
    #[must_use]
    pub const fn contains(&self, id: i32) -> bool {
        match id & !0x3f {
            0 => (self.m_low & (1u64 << id)) != 0,
            128 => (self.m_mut & (1u64 << (id - 128))) != 0,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_contains_low_range() {
        let mut s = BiomeSet::new();
        s.add(0); // ocean
        s.add(63);
        assert!(s.contains(0));
        assert!(s.contains(63));
        assert!(!s.contains(1));
        assert!(!s.contains(64));
        assert!(!s.contains(128));
    }

    #[test]
    fn add_and_contains_mutated_range() {
        let mut s = BiomeSet::new();
        s.add(128);
        s.add(168); // bamboo_jungle… wait, 168 is `id & !0x3f` = 128 (since 168 = 0xA8, mask = 0x80). So 168 lands in m_mut at bit 40.
        // Mirror cubiomes: 168 & ~0x3f = 168 & 0xffffffc0 = 128. Goes to m_mut bit (168-128)=40.
        assert!(s.contains(128));
        assert!(s.contains(168));
        // 191 (= 128 + 63) is the upper edge.
        s.add(191);
        assert!(s.contains(191));
        // 192 falls in the "ignored" range.
        s.add(192);
        assert!(!s.contains(192));
    }

    #[test]
    fn out_of_range_ids_ignored() {
        let mut s = BiomeSet::new();
        s.add(64);
        s.add(127);
        s.add(200);
        assert_eq!(s.m_low, 0);
        assert_eq!(s.m_mut, 0);
        assert!(!s.contains(64));
    }
}
