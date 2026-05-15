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
    /// `desert` (biome id 2).
    pub const DESERT: Self = Self(2);
    /// `mountains` / `extreme_hills` (biome id 3).
    pub const MOUNTAINS: Self = Self(3);
    /// `forest` (biome id 4 in `cubiomes/biomes.h`).
    pub const FOREST: Self = Self(4);
    /// `taiga` (biome id 5).
    pub const TAIGA: Self = Self(5);
    /// `swamp` / `swampland` (biome id 6).
    pub const SWAMP: Self = Self(6);
    /// `river` (biome id 7).
    pub const RIVER: Self = Self(7);
    /// `frozen_ocean` (biome id 10).
    pub const FROZEN_OCEAN: Self = Self(10);
    /// `frozen_river` (biome id 11).
    pub const FROZEN_RIVER: Self = Self(11);
    /// `snowy_tundra` (biome id 12).
    pub const SNOWY_TUNDRA: Self = Self(12);
    /// `mushroom_fields` (biome id 14).
    pub const MUSHROOM_FIELDS: Self = Self(14);
    /// `beach` (biome id 16).
    pub const BEACH: Self = Self(16);
    /// `jungle` (biome id 21).
    pub const JUNGLE: Self = Self(21);
    /// `jungle_hills` (biome id 22).
    pub const JUNGLE_HILLS: Self = Self(22);
    /// `bamboo_jungle` (biome id 168).
    pub const BAMBOO_JUNGLE: Self = Self(168);
    /// `sunflower_plains` (biome id 129; `plains + 128`).
    pub const SUNFLOWER_PLAINS: Self = Self(129);
    /// `deep_ocean` (biome id 24).
    pub const DEEP_OCEAN: Self = Self(24);
    /// `stone_shore` (biome id 25).
    pub const STONE_SHORE: Self = Self(25);
    /// `snowy_beach` (biome id 26).
    pub const SNOWY_BEACH: Self = Self(26);
    /// `birch_forest` (biome id 27).
    pub const BIRCH_FOREST: Self = Self(27);
    /// `dark_forest` (biome id 29).
    pub const DARK_FOREST: Self = Self(29);
    /// `snowy_taiga` (biome id 30).
    pub const SNOWY_TAIGA: Self = Self(30);
    /// `giant_tree_taiga` / `megaTaiga` (biome id 32).
    pub const GIANT_TREE_TAIGA: Self = Self(32);
    /// `savanna` (biome id 35).
    pub const SAVANNA: Self = Self(35);
    /// `badlands` / `mesa` (biome id 37).
    pub const BADLANDS: Self = Self(37);
    /// `wooded_badlands_plateau` (biome id 38).
    pub const WOODED_BADLANDS_PLATEAU: Self = Self(38);
    /// `badlands_plateau` (biome id 39).
    pub const BADLANDS_PLATEAU: Self = Self(39);
    /// `warm_ocean` (biome id 44).
    pub const WARM_OCEAN: Self = Self(44);
    /// `lukewarm_ocean` (biome id 45).
    pub const LUKEWARM_OCEAN: Self = Self(45);
    /// `cold_ocean` (biome id 46).
    pub const COLD_OCEAN: Self = Self(46);
    /// `deep_warm_ocean` (biome id 47).
    pub const DEEP_WARM_OCEAN: Self = Self(47);
    /// `deep_lukewarm_ocean` (biome id 48).
    pub const DEEP_LUKEWARM_OCEAN: Self = Self(48);
    /// `deep_cold_ocean` (biome id 49).
    pub const DEEP_COLD_OCEAN: Self = Self(49);
    /// `deep_frozen_ocean` (biome id 50).
    pub const DEEP_FROZEN_OCEAN: Self = Self(50);

    /// Returns `true` if `id` is one of the five shallow-ocean variants
    /// (`ocean`, `frozen_ocean`, `cold_ocean`, `lukewarm_ocean`,
    /// `warm_ocean`). Mirrors cubiomes' `isShallowOcean` helper.
    #[inline]
    #[must_use]
    pub const fn is_shallow_ocean_id(id: i32) -> bool {
        matches!(id, 0 | 10 | 44 | 45 | 46)
    }

    /// Returns `true` if `id` is one of the five deep-ocean variants
    /// (`deep_ocean`, `deep_warm_ocean`, `deep_lukewarm_ocean`,
    /// `deep_cold_ocean`, `deep_frozen_ocean`). Mirrors cubiomes'
    /// `isDeepOcean`.
    #[inline]
    #[must_use]
    pub const fn is_deep_ocean_id(id: i32) -> bool {
        matches!(id, 24 | 47 | 48 | 49 | 50)
    }

    /// Returns `true` if `id` is any ocean variant — shallow or deep.
    /// Mirrors cubiomes' `isOceanic`.
    #[inline]
    #[must_use]
    pub const fn is_oceanic_id(id: i32) -> bool {
        matches!(id, 0 | 10 | 24 | 44 | 45 | 46 | 47 | 48 | 49 | 50)
    }

    /// Returns `true` if `id` is one of the nine "snowy" biomes.
    /// Mirrors cubiomes' `isSnowy`.
    #[inline]
    #[must_use]
    pub const fn is_snowy_id(id: i32) -> bool {
        matches!(
            id,
            10  // frozen_ocean
            | 11  // frozen_river
            | 12  // snowy_tundra
            | 13  // snowy_mountains
            | 26  // snowy_beach
            | 30  // snowy_taiga
            | 31  // snowy_taiga_hills
            | 140 // ice_spikes
            | 158 // snowy_taiga_mountains
        )
    }

    /// Returns `true` if `id` is one of the six "mesa" / badlands
    /// biome variants. Mirrors cubiomes' `isMesa`.
    #[inline]
    #[must_use]
    pub const fn is_mesa_id(id: i32) -> bool {
        matches!(
            id,
            37  // badlands
            | 38  // wooded_badlands_plateau
            | 39  // badlands_plateau
            | 165 // eroded_badlands
            | 166 // modified_wooded_badlands_plateau
            | 167 // modified_badlands_plateau
        )
    }

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
