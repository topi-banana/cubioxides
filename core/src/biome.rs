//! Biome identifier newtype and a handful of sentinel constants.
//!
//! cubiomes represents biomes as plain `int` values matching the
//! `enum BiomeID` in `cubiomes/biomes.h`. We wrap them in a transparent
//! newtype so that the layer pipeline cannot accidentally mix biome IDs
//! with raw layer outputs or seed material.
//!
//! The exhaustive `BiomeID` enum will land alongside the per-version
//! layer porting work; this initial module exposes only the values used
//! by the M3.1 layers (`Biome::NONE`, `Biome::OCEAN`, `Biome::PLAINS`).

/// Transparent wrapper around cubiomes' `int` biome identifier.
///
/// `Biome(-1)` represents "no biome" (matches cubiomes' `none = -1`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Biome(pub i32);

impl Biome {
    /// Sentinel for "no biome assigned yet" (cubiomes `none = -1`).
    pub const NONE: Self = Self(-1);
    /// `ocean` from cubiomes/biomes.h.
    pub const OCEAN: Self = Self(0);
    /// `plains`.
    pub const PLAINS: Self = Self(1);
    /// `forest` (biome id 4 in `cubiomes/biomes.h`).
    pub const FOREST: Self = Self(4);

    /// Underlying signed integer ID.
    #[inline]
    #[must_use]
    pub const fn id(self) -> i32 {
        self.0
    }
}

impl From<i32> for Biome {
    #[inline]
    fn from(id: i32) -> Self {
        Self(id)
    }
}

impl From<Biome> for i32 {
    #[inline]
    fn from(b: Biome) -> Self {
        b.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sentinels_match_cubiomes() {
        assert_eq!(Biome::NONE.id(), -1);
        assert_eq!(Biome::OCEAN.id(), 0);
        assert_eq!(Biome::PLAINS.id(), 1);
    }

    #[test]
    fn round_trip_through_i32() {
        let b: Biome = 42.into();
        assert_eq!(i32::from(b), 42);
    }

    #[test]
    fn default_is_ocean() {
        // Default is the zero value, which is `ocean` in cubiomes.
        assert_eq!(Biome::default(), Biome::OCEAN);
    }
}
