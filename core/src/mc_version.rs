//! Minecraft Java Edition version and dimension identifiers.
//!
//! [`MCVersion`] mirrors the `enum MCVersion` defined in `cubiomes/biomes.h`.
//! The variants are kept in the same discriminant order as upstream so that
//! `as u8` comparisons line up with the C implementation. Each variant
//! corresponds to the *latest patch* of a major release, matching the
//! cubiomes convention that development effort focuses on the newest patch.

/// Minecraft Java Edition version, in the order cubiomes uses internally.
///
/// The order is significant: it matches the `enum MCVersion` discriminants
/// from `cubiomes/biomes.h`. Use [`MCVersion::is_at_least`] to compare
/// versions rather than relying on the numeric representation directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
#[repr(u8)]
pub enum MCVersion {
    /// Undefined / unspecified version. Used as a sentinel only.
    Undef = 0,
    /// Beta 1.7.
    B1_7,
    /// Beta 1.8.
    B1_8,
    /// 1.0.0 (release).
    V1_0,
    /// 1.1.
    V1_1,
    /// 1.2.5.
    V1_2,
    /// 1.3.2.
    V1_3,
    /// 1.4.7.
    V1_4,
    /// 1.5.2.
    V1_5,
    /// 1.6.4.
    V1_6,
    /// 1.7.10.
    V1_7,
    /// 1.8.9.
    V1_8,
    /// 1.9.4.
    V1_9,
    /// 1.10.2.
    V1_10,
    /// 1.11.2.
    V1_11,
    /// 1.12.2.
    V1_12,
    /// 1.13.2.
    V1_13,
    /// 1.14.4.
    V1_14,
    /// 1.15.2.
    V1_15,
    /// 1.16.1 — distinct from 1.16.5 because of nether biome generation changes.
    V1_16_1,
    /// 1.16.5 — the canonical "1.16".
    V1_16,
    /// 1.17.1.
    V1_17,
    /// 1.18.2 — first version with noise-based biome generation.
    V1_18,
    /// 1.19.2 — distinct from 1.19.4 (mangrove swamp + cherry grove diff).
    V1_19_2,
    /// 1.19.4 — the canonical "1.19".
    V1_19,
    /// 1.20.6 — the canonical "1.20".
    V1_20,
    /// 1.21.1.
    V1_21_1,
    /// 1.21.3.
    V1_21_3,
    /// 1.21 Winter Drop — exact version TBA upstream; matches cubiomes' `MC_1_21_WD`.
    V1_21,
}

impl MCVersion {
    /// The newest version cubioxides knows about.
    ///
    /// Tracks `MC_NEWEST` from cubiomes.
    pub const NEWEST: Self = Self::V1_21;

    /// Numeric ordinal in the cubiomes enum ordering.
    ///
    /// Useful for `<` / `<=` comparisons in const contexts where deriving
    /// [`PartialOrd`] does not yet help.
    #[inline]
    #[must_use]
    pub const fn ord(self) -> u8 {
        self as u8
    }

    /// Returns `true` if `self` is the same as or newer than `other`.
    ///
    /// Mirrors the `mc >= MC_1_X` checks scattered throughout cubiomes.
    #[inline]
    #[must_use]
    pub const fn is_at_least(self, other: Self) -> bool {
        self.ord() >= other.ord()
    }

    /// Returns `true` if `self` is strictly older than `other`.
    #[inline]
    #[must_use]
    pub const fn is_before(self, other: Self) -> bool {
        self.ord() < other.ord()
    }
}

/// Minecraft dimension identifier.
///
/// The discriminant values are chosen to match cubiomes' `enum Dimension`
/// so that round-tripping through `i32` preserves identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
#[repr(i8)]
pub enum Dimension {
    /// The Nether.
    Nether = -1,
    /// The Overworld.
    Overworld = 0,
    /// The End.
    End = 1,
}

impl Dimension {
    /// Cubiomes-compatible signed integer representation.
    ///
    /// Mirrors the values stored in `Generator::dim` in the C source.
    #[inline]
    #[must_use]
    pub const fn as_i32(self) -> i32 {
        self as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_ord_matches_cubiomes_enum() {
        // Discriminants must line up with cubiomes/biomes.h `enum MCVersion`
        // exactly, so `as u8` round-trips through the FFI layer.
        assert_eq!(MCVersion::Undef.ord(), 0);
        assert_eq!(MCVersion::B1_7.ord(), 1);
        assert_eq!(MCVersion::B1_8.ord(), 2);
        assert_eq!(MCVersion::V1_0.ord(), 3);
        assert_eq!(MCVersion::V1_16_1.ord(), 19);
        assert_eq!(MCVersion::V1_16.ord(), 20);
        assert_eq!(MCVersion::V1_17.ord(), 21);
        assert_eq!(MCVersion::V1_18.ord(), 22);
        assert_eq!(MCVersion::V1_19_2.ord(), 23);
        assert_eq!(MCVersion::V1_19.ord(), 24);
        assert_eq!(MCVersion::V1_20.ord(), 25);
        assert_eq!(MCVersion::V1_21_1.ord(), 26);
        assert_eq!(MCVersion::V1_21_3.ord(), 27);
        assert_eq!(MCVersion::V1_21.ord(), 28);
    }

    #[test]
    fn is_at_least_handles_typical_comparisons() {
        assert!(MCVersion::V1_18.is_at_least(MCVersion::V1_13));
        assert!(MCVersion::V1_13.is_at_least(MCVersion::V1_13));
        assert!(!MCVersion::V1_12.is_at_least(MCVersion::V1_13));
        assert!(MCVersion::V1_21.is_at_least(MCVersion::V1_18));
    }

    #[test]
    fn is_before_is_strict_inverse() {
        assert!(MCVersion::V1_12.is_before(MCVersion::V1_13));
        assert!(!MCVersion::V1_13.is_before(MCVersion::V1_13));
    }

    #[test]
    fn newest_is_v1_21() {
        assert_eq!(MCVersion::NEWEST, MCVersion::V1_21);
    }

    #[test]
    fn dimension_as_i32_matches_cubiomes() {
        assert_eq!(Dimension::Nether.as_i32(), -1);
        assert_eq!(Dimension::Overworld.as_i32(), 0);
        assert_eq!(Dimension::End.as_i32(), 1);
    }

    #[test]
    fn version_ordering_through_partial_ord_matches_ord() {
        // Sanity: derived PartialOrd should agree with our ord() helper,
        // because both rely on the declaration order of the enum.
        assert!(MCVersion::V1_18 > MCVersion::V1_17);
        assert!(MCVersion::V1_17 < MCVersion::V1_18);
    }
}
