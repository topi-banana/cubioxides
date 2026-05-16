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
    /// `mushroom_field_shore` (biome id 15).
    pub const MUSHROOM_FIELD_SHORE: Self = Self(15);
    /// `beach` (biome id 16).
    pub const BEACH: Self = Self(16);
    /// `jungle` (biome id 21).
    pub const JUNGLE: Self = Self(21);
    /// `jungle_hills` (biome id 22).
    pub const JUNGLE_HILLS: Self = Self(22);
    /// `jungle_edge` (biome id 23).
    pub const JUNGLE_EDGE: Self = Self(23);
    /// `bamboo_jungle` (biome id 168).
    pub const BAMBOO_JUNGLE: Self = Self(168);
    /// `mountain_edge` / `extremeHillsEdge` (biome id 20).
    pub const MOUNTAIN_EDGE: Self = Self(20);
    /// `wooded_mountains` (biome id 34).
    pub const WOODED_MOUNTAINS: Self = Self(34);
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
    /// `nether_wastes` / `hell` (biome id 8).
    pub const NETHER_WASTES: Self = Self(8);
    /// `the_end` / `sky` (biome id 9).
    pub const THE_END: Self = Self(9);
    /// `soul_sand_valley` (biome id 170; 1.16+).
    pub const SOUL_SAND_VALLEY: Self = Self(170);
    /// `crimson_forest` (biome id 171; 1.16+).
    pub const CRIMSON_FOREST: Self = Self(171);
    /// `warped_forest` (biome id 172; 1.16+).
    pub const WARPED_FOREST: Self = Self(172);
    /// `basalt_deltas` (biome id 173; 1.16+).
    pub const BASALT_DELTAS: Self = Self(173);
    /// `small_end_islands` (biome id 40; 1.13+).
    pub const SMALL_END_ISLANDS: Self = Self(40);
    /// `end_midlands` (biome id 41; 1.13+).
    pub const END_MIDLANDS: Self = Self(41);
    /// `end_highlands` (biome id 42; 1.13+).
    pub const END_HIGHLANDS: Self = Self(42);
    /// `end_barrens` (biome id 43; 1.13+).
    pub const END_BARRENS: Self = Self(43);
    /// `seasonal_forest` (biome id 51; Alpha 1.2 – Beta 1.7 only).
    pub const SEASONAL_FOREST: Self = Self(51);
    /// `rainforest` (biome id 52; Alpha 1.2 – Beta 1.7 only).
    pub const RAINFOREST: Self = Self(52);
    /// `shrubland` (biome id 53; Alpha 1.2 – Beta 1.7 only).
    pub const SHRUBLAND: Self = Self(53);

    /// `the_void` (biome id 127; 1.9+).
    pub const THE_VOID: Self = Self(127);
    /// `dripstone_caves` (biome id 174; 1.17+).
    pub const DRIPSTONE_CAVES: Self = Self(174);
    /// `lush_caves` (biome id 175; 1.17+).
    pub const LUSH_CAVES: Self = Self(175);
    /// `meadow` (biome id 177; 1.18+).
    pub const MEADOW: Self = Self(177);
    /// `grove` (biome id 178; 1.18+).
    pub const GROVE: Self = Self(178);
    /// `snowy_slopes` (biome id 179; 1.18+).
    pub const SNOWY_SLOPES: Self = Self(179);
    /// `jagged_peaks` (biome id 180; 1.18+).
    pub const JAGGED_PEAKS: Self = Self(180);
    /// `frozen_peaks` (biome id 181; 1.18+).
    pub const FROZEN_PEAKS: Self = Self(181);
    /// `stony_peaks` (biome id 182; 1.18+).
    pub const STONY_PEAKS: Self = Self(182);
    /// `deep_dark` (biome id 183; 1.19.2+).
    pub const DEEP_DARK: Self = Self(183);
    /// `mangrove_swamp` (biome id 184; 1.19.2+).
    pub const MANGROVE_SWAMP: Self = Self(184);
    /// `cherry_grove` (biome id 185; 1.20+).
    pub const CHERRY_GROVE: Self = Self(185);
    /// `pale_garden` (biome id 186; 1.21 WD+).
    pub const PALE_GARDEN: Self = Self(186);

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
    ///
    /// # Example
    ///
    /// ```
    /// use cubioxides::biome::Biome;
    ///
    /// // The 10 ocean variants (5 shallow + 5 deep) all qualify;
    /// // river / beach / land biomes do not — note that the shallow
    /// // and deep families remain individually addressable via
    /// // [`Biome::is_shallow_ocean_id`] / [`Biome::is_deep_ocean_id`].
    /// assert!(Biome::is_oceanic_id(Biome::OCEAN.id()));         // shallow
    /// assert!(Biome::is_oceanic_id(Biome::DEEP_OCEAN.id()));    // deep
    /// assert!(Biome::is_oceanic_id(Biome::WARM_OCEAN.id()));    // temperature variant
    /// assert!(!Biome::is_oceanic_id(Biome::RIVER.id()));
    /// assert!(!Biome::is_oceanic_id(Biome::BEACH.id()));
    /// assert!(!Biome::is_oceanic_id(Biome::PLAINS.id()));
    /// ```
    #[inline]
    #[must_use]
    pub const fn is_oceanic_id(id: i32) -> bool {
        matches!(id, 0 | 10 | 24 | 44 | 45 | 46 | 47 | 48 | 49 | 50)
    }

    /// `getDimension(id)` — return the dimension a biome belongs to.
    /// Bit-exact port of `cubiomes/biomes.c::getDimension`.
    ///
    /// - `[40, 43]` (`small_end_islands` ..= `end_barrens`) → End
    /// - `[170, 173]` (`soul_sand_valley` ..= `basalt_deltas`) → Nether
    /// - `9` (`the_end`) → End
    /// - `8` (`nether_wastes`) → Nether
    /// - else → Overworld (including `none = -1`).
    #[inline]
    #[must_use]
    pub const fn dimension_id(id: i32) -> crate::mc_version::Dimension {
        use crate::mc_version::Dimension;
        if id >= 40 && id <= 43 {
            return Dimension::End;
        }
        if id >= 170 && id <= 173 {
            return Dimension::Nether;
        }
        if id == 9 {
            return Dimension::End;
        }
        if id == 8 {
            return Dimension::Nether;
        }
        Dimension::Overworld
    }

    /// `biome2str(mc, id)` — return the human-readable biome name,
    /// or `None` if `id` is not a known biome. For 1.18+ the
    /// "renamed" biomes (`snowy_tundra` → `snowy_plains`, etc.)
    /// return their 1.18-and-later names; pre-1.18 returns the
    /// legacy name. Bit-exact port of `cubiomes/util.c::biome2str`.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn name(mc: crate::mc_version::MCVersion, id: i32) -> Option<&'static str> {
        if mc.is_at_least(crate::mc_version::MCVersion::V1_18) {
            // 1.18+ renamed biomes (id is shared with the pre-1.18 name).
            match id {
                155 => return Some("old_growth_birch_forest"), // tall_birch_forest
                32 => return Some("old_growth_pine_taiga"),    // giant_tree_taiga
                160 => return Some("old_growth_spruce_taiga"), // giant_spruce_taiga
                12 => return Some("snowy_plains"),             // snowy_tundra
                23 => return Some("sparse_jungle"),            // jungle_edge
                25 => return Some("stony_shore"),              // stone_shore
                3 => return Some("windswept_hills"),           // mountains
                34 => return Some("windswept_forest"),         // wooded_mountains
                131 => return Some("windswept_gravelly_hills"), // gravelly_mountains
                163 => return Some("windswept_savanna"),       // shattered_savanna
                38 => return Some("wooded_badlands"),          // wooded_badlands_plateau
                _ => {}
            }
        }
        Some(match id {
            0 => "ocean",
            1 => "plains",
            2 => "desert",
            3 => "mountains",
            4 => "forest",
            5 => "taiga",
            6 => "swamp",
            7 => "river",
            8 => "nether_wastes",
            9 => "the_end",
            10 => "frozen_ocean",
            11 => "frozen_river",
            12 => "snowy_tundra",
            13 => "snowy_mountains",
            14 => "mushroom_fields",
            15 => "mushroom_field_shore",
            16 => "beach",
            17 => "desert_hills",
            18 => "wooded_hills",
            19 => "taiga_hills",
            20 => "mountain_edge",
            21 => "jungle",
            22 => "jungle_hills",
            23 => "jungle_edge",
            24 => "deep_ocean",
            25 => "stone_shore",
            26 => "snowy_beach",
            27 => "birch_forest",
            28 => "birch_forest_hills",
            29 => "dark_forest",
            30 => "snowy_taiga",
            31 => "snowy_taiga_hills",
            32 => "giant_tree_taiga",
            33 => "giant_tree_taiga_hills",
            34 => "wooded_mountains",
            35 => "savanna",
            36 => "savanna_plateau",
            37 => "badlands",
            38 => "wooded_badlands_plateau",
            39 => "badlands_plateau",
            40 => "small_end_islands",
            41 => "end_midlands",
            42 => "end_highlands",
            43 => "end_barrens",
            44 => "warm_ocean",
            45 => "lukewarm_ocean",
            46 => "cold_ocean",
            47 => "deep_warm_ocean",
            48 => "deep_lukewarm_ocean",
            49 => "deep_cold_ocean",
            50 => "deep_frozen_ocean",
            // Alpha 1.2 – Beta 1.7
            51 => "seasonal_forest",
            52 => "rainforest",
            53 => "shrubland",
            127 => "the_void",
            // mutated variants
            129 => "sunflower_plains",
            130 => "desert_lakes",
            131 => "gravelly_mountains",
            132 => "flower_forest",
            133 => "taiga_mountains",
            134 => "swamp_hills",
            140 => "ice_spikes",
            149 => "modified_jungle",
            151 => "modified_jungle_edge",
            155 => "tall_birch_forest",
            156 => "tall_birch_hills",
            157 => "dark_forest_hills",
            158 => "snowy_taiga_mountains",
            160 => "giant_spruce_taiga",
            161 => "giant_spruce_taiga_hills",
            162 => "modified_gravelly_mountains",
            163 => "shattered_savanna",
            164 => "shattered_savanna_plateau",
            165 => "eroded_badlands",
            166 => "modified_wooded_badlands_plateau",
            167 => "modified_badlands_plateau",
            // 1.14
            168 => "bamboo_jungle",
            169 => "bamboo_jungle_hills",
            // 1.16
            170 => "soul_sand_valley",
            171 => "crimson_forest",
            172 => "warped_forest",
            173 => "basalt_deltas",
            // 1.17
            174 => "dripstone_caves",
            175 => "lush_caves",
            // 1.18
            177 => "meadow",
            178 => "grove",
            179 => "snowy_slopes",
            180 => "jagged_peaks",
            181 => "frozen_peaks",
            182 => "stony_peaks",
            // 1.19
            183 => "deep_dark",
            184 => "mangrove_swamp",
            // 1.20
            185 => "cherry_grove",
            // 1.21 Winter Drop
            186 => "pale_garden",
            _ => return None,
        })
    }

    /// `biomeExists(mc, id)` — does this biome exist in the given
    /// MC version? Bit-exact port of `cubiomes/biomes.c::biomeExists`.
    /// Drives both `is_overworld_id` and the stronghold-biome mask.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn biome_exists(mc: crate::mc_version::MCVersion, id: i32) -> bool {
        use crate::mc_version::MCVersion;
        if mc.is_at_least(MCVersion::V1_18) {
            // 1.18+: explicit allowlist plus Nether / End contiguous ranges.
            if (170..=173).contains(&id) || (40..=43).contains(&id) {
                return true;
            }
            if id == 186 {
                return mc.is_at_least(MCVersion::V1_21);
            }
            if id == 185 {
                return mc.is_at_least(MCVersion::V1_20);
            }
            if id == 183 || id == 184 {
                return mc.is_at_least(MCVersion::V1_19_2);
            }
            return matches!(
                id,
                0 | 1
                    | 2
                    | 3
                    | 4
                    | 5
                    | 6
                    | 7
                    | 8
                    | 9
                    | 10
                    | 11
                    | 12
                    | 14
                    | 16
                    | 21
                    | 23
                    | 24
                    | 25
                    | 26
                    | 27
                    | 29
                    | 30
                    | 32
                    | 34
                    | 35
                    | 36
                    | 37
                    | 38
                    | 44
                    | 45
                    | 46
                    | 47
                    | 48
                    | 49
                    | 50
                    | 129
                    | 131
                    | 132
                    | 140
                    | 155
                    | 160
                    | 163
                    | 165
                    | 168
                    | 174
                    | 175
                    | 177
                    | 178
                    | 179
                    | 180
                    | 181
                    | 182
            );
        }
        if !mc.is_at_least(MCVersion::B1_8) {
            // <= B1.7 — alpha/beta only.
            return matches!(id, 0 | 1 | 2 | 4 | 5 | 6 | 10 | 12 | 35 | 51 | 52 | 53);
        }
        if !mc.is_at_least(MCVersion::V1_0) {
            // B1.8: extra exclusions.
            if matches!(id, 10 | 11 | 12 | 14 | 15 | 9) {
                return false;
            }
        }
        if !mc.is_at_least(MCVersion::V1_1) {
            // 1.0 also excludes the_end (9), but it's already excluded above.
            if matches!(id, 13 | 16 | 17 | 18 | 19 | 20) {
                return false;
            }
        }

        // General range checks.
        // ocean..mountain_edge = 0..20: always (subject to above exclusions).
        if (0..=20).contains(&id) {
            return true;
        }
        // jungle..jungle_hills = 21..22: 1.2+
        if (21..=22).contains(&id) {
            return mc.is_at_least(MCVersion::V1_2);
        }
        // jungle_edge..badlands_plateau = 23..39: 1.7+
        if (23..=39).contains(&id) {
            return mc.is_at_least(MCVersion::V1_7);
        }
        // small_end_islands..end_barrens = 40..43: 1.9+
        if (40..=43).contains(&id) {
            return mc.is_at_least(MCVersion::V1_9);
        }
        // warm_ocean..deep_frozen_ocean = 44..50: 1.13+
        if (44..=50).contains(&id) {
            return mc.is_at_least(MCVersion::V1_13);
        }

        match id {
            127 => mc.is_at_least(MCVersion::V1_9), // the_void
            129 | 130 | 131 | 132 | 133 | 134 | 140 | 149 | 151 | 155 | 156 | 157 | 158 | 160
            | 161 | 162 | 163 | 164 | 165 | 166 | 167 => {
                // mutated variants — 1.7+
                mc.is_at_least(MCVersion::V1_7)
            }
            168 | 169 => mc.is_at_least(MCVersion::V1_14),
            170..=173 => mc.is_at_least(MCVersion::V1_16_1),
            174 | 175 => mc.is_at_least(MCVersion::V1_17),
            _ => false,
        }
    }

    /// `isOverworld(mc, id)` — predicate `biomeExists && id is in
    /// the Overworld dimension`. Bit-exact port of cubiomes'
    /// `isOverworld`.
    #[must_use]
    pub fn is_overworld_id(mc: crate::mc_version::MCVersion, id: i32) -> bool {
        use crate::mc_version::MCVersion;
        if !Self::biome_exists(mc, id) {
            return false;
        }
        // End + Nether ranges.
        if (40..=43).contains(&id) || (170..=173).contains(&id) {
            return false;
        }
        match id {
            // nether_wastes / the_end / deep_warm_ocean / the_void
            8 | 9 | 47 | 127 => false,
            10 => !mc.is_at_least(MCVersion::V1_7) || mc.is_at_least(MCVersion::V1_13), // frozen_ocean
            20 => !mc.is_at_least(MCVersion::V1_7), // mountain_edge
            155 => !mc.is_at_least(MCVersion::V1_9) || mc.is_at_least(MCVersion::V1_11), // tall_birch_forest
            174 | 175 => mc.is_at_least(MCVersion::V1_18), // dripstone/lush caves
            _ => true,
        }
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

    /// Map a biome ID to its "category" ID — a representative biome ID
    /// for the family the input belongs to. Bit-exact port of cubiomes'
    /// `getCategory` in biomes.c. Returns `-1` (none) for unrecognised
    /// IDs.
    #[inline]
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub const fn get_category_id(mc: crate::mc_version::MCVersion, id: i32) -> i32 {
        match id {
            // beach / snowy_beach.
            16 | 26 => 16,
            // desert family.
            2 | 17 | 130 => 2,
            // mountains family.
            3 | 20 | 34 | 131 | 162 => 3,
            // forest family.
            4 | 18 | 27 | 28 | 29 | 132 | 155 | 156 | 157 => 4,
            // snowy tundra family.
            12 | 13 | 140 => 12,
            // jungle family.
            21 | 22 | 23 | 149 | 151 | 168 | 169 => 21,
            // mesa / badlands family (canonical category 37).
            37 | 165 | 166 | 167 => 37,
            // wooded_badlands_plateau / badlands_plateau — pre-1.16
            // collapses to mesa, 1.16+ keeps the plateau distinction.
            // cubiomes' check is `mc <= MC_1_15`, so V1_16_1 onward
            // (ord 19) keeps the plateau category, NOT V1_16 (= V1_16_5,
            // ord 20).
            38 | 39 => {
                if mc.is_at_least(crate::mc_version::MCVersion::V1_16_1) {
                    39
                } else {
                    37
                }
            }
            // mushroom_fields / mushroom_field_shore.
            14 | 15 => 14,
            // stone_shore is its own category.
            25 => 25,
            // every ocean variant collapses to plain `ocean`.
            0 | 10 | 24 | 44 | 45 | 46 | 47 | 48 | 49 | 50 => 0,
            // plains / sunflower_plains.
            1 | 129 => 1,
            // river / frozen_river.
            7 | 11 => 7,
            // savanna family.
            35 | 36 | 163 | 164 => 35,
            // swamp / swamp_hills.
            6 | 134 => 6,
            // taiga family.
            5 | 19 | 30 | 31 | 32 | 33 | 133 | 158 | 160 | 161 => 5,
            // nether_wastes family (1.16+).
            8 | 170 | 171 | 172 | 173 => 8,
            _ => -1,
        }
    }

    /// Return the "mutated" variant of `id`. Bit-exact port of
    /// cubiomes' `getMutated`. Returns `-1` (none) when no mutation
    /// exists. The `birch_forest` / `birch_forest_hills` cases depend
    /// on MC version (1.9 / 1.10 swap the mapping to emulate
    /// MC-98995).
    #[inline]
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub const fn get_mutated_id(mc: crate::mc_version::MCVersion, id: i32) -> i32 {
        match id {
            1 => 129,  // plains -> sunflower_plains
            2 => 130,  // desert -> desert_lakes
            3 => 131,  // mountains -> gravelly_mountains
            4 => 132,  // forest -> flower_forest
            5 => 133,  // taiga -> taiga_mountains
            6 => 134,  // swamp -> swamp_hills
            12 => 140, // snowy_tundra -> ice_spikes
            21 => 149, // jungle -> modified_jungle
            23 => 151, // jungle_edge -> modified_jungle_edge
            27 => {
                // birch_forest -> tall_birch_hills (1.9-1.10) or tall_birch_forest
                if mc.is_at_least(crate::mc_version::MCVersion::V1_9)
                    && !mc.is_at_least(crate::mc_version::MCVersion::V1_11)
                {
                    156
                } else {
                    155
                }
            }
            28 => {
                // birch_forest_hills -> none (1.9-1.10) or tall_birch_hills
                if mc.is_at_least(crate::mc_version::MCVersion::V1_9)
                    && !mc.is_at_least(crate::mc_version::MCVersion::V1_11)
                {
                    -1
                } else {
                    156
                }
            }
            29 => 157, // dark_forest -> dark_forest_hills
            30 => 158, // snowy_taiga -> snowy_taiga_mountains
            32 => 160, // giant_tree_taiga -> giant_spruce_taiga
            33 => 161, // giant_tree_taiga_hills -> giant_spruce_taiga_hills
            34 => 162, // wooded_mountains -> modified_gravelly_mountains
            35 => 163, // savanna -> shattered_savanna
            36 => 164, // savanna_plateau -> shattered_savanna_plateau
            37 => 165, // badlands -> eroded_badlands
            38 => 166, // wooded_badlands_plateau -> modified_wooded_badlands_plateau
            39 => 167, // badlands_plateau -> modified_badlands_plateau
            _ => -1,
        }
    }

    /// `true` if `id1` and `id2` belong to the same biome family.
    /// Bit-exact port of cubiomes' `areSimilar`.
    #[inline]
    #[must_use]
    pub const fn are_similar_ids(mc: crate::mc_version::MCVersion, id1: i32, id2: i32) -> bool {
        if id1 == id2 {
            return true;
        }
        // Cubiomes: `mc <= MC_1_15` — the badlands_plateau /
        // wooded_badlands_plateau pair is considered similar only
        // before 1.16.1. Rust's `is_before(V1_16_1)` mirrors this
        // exactly (V1_16_1 is the version just after V1_15).
        if mc.is_before(crate::mc_version::MCVersion::V1_16_1) && (id1 == 38 || id1 == 39) {
            return id2 == 38 || id2 == 39;
        }
        Self::get_category_id(mc, id1) == Self::get_category_id(mc, id2)
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

/// Cubiomes' `getCategory(mc, id)` — map a biome ID to its
/// "category" representative biome ID. Returns the cubiomes
/// `none = -1` sentinel for biomes with no category.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn get_category(mc: crate::mc_version::MCVersion, id: i32) -> i32 {
    match id {
        // beach / snowy_beach -> beach (16)
        16 | 26 => 16,
        // desert / desert_hills / desert_lakes -> desert (2)
        2 | 17 | 130 => 2,
        // mountains family -> mountains (3)
        3 | 20 | 34 | 131 | 162 => 3,
        // forest family -> forest (4)
        4 | 18 | 27 | 28 | 29 | 132 | 155 | 156 | 157 => 4,
        // snowy_tundra / snowy_mountains / ice_spikes -> snowy_tundra (12)
        12 | 13 | 140 => 12,
        // jungle family -> jungle (21)
        21 | 22 | 23 | 149 | 151 | 168 | 169 => 21,
        // mesa / badlands family
        37 | 165 | 166 | 167 => 37,
        // 1.15-: -> mesa(37); 1.16+: keep distinction.
        38 | 39 => {
            if mc.is_at_least(crate::mc_version::MCVersion::V1_16_1) {
                39 // badlands_plateau
            } else {
                37 // mesa
            }
        }
        // mushroom_fields / mushroom_field_shore -> mushroom_fields (14)
        14 | 15 => 14,
        // stone_shore -> stone_shore (25)
        25 => 25,
        // ocean family -> ocean (0)
        0 | 10 | 24 | 44 | 45 | 46 | 47 | 48 | 49 | 50 => 0,
        // plains / sunflower_plains -> plains (1)
        1 | 129 => 1,
        // river / frozen_river -> river (7)
        7 | 11 => 7,
        // savanna family -> savanna (35)
        35 | 36 | 163 | 164 => 35,
        // swamp family -> swamp (6)
        6 | 134 => 6,
        // taiga family -> taiga (5)
        5 | 19 | 30 | 31 | 32 | 33 | 133 | 158 | 160 | 161 => 5,
        // nether family -> nether_wastes (8)
        8 | 170 | 171 | 172 | 173 => 8,
        _ => -1,
    }
}

/// Cubiomes' `getBiomeDepthAndScale` output triple. `grass` is the
/// minimum surface block-Y for grass placement (0 means the biome
/// doesn't produce grass; 60–64 are typical for shores / rivers;
/// 62 is the default for most overworld biomes).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BiomeDepthScale {
    /// Terrain depth offset (cubiomes' `d`).
    pub depth: f64,
    /// Terrain noise amplitude (cubiomes' `s`).
    pub scale: f64,
    /// Minimum spawn-grass height (cubiomes' `g`).
    pub grass: i32,
}

/// `getBiomeDepthAndScale(id, depth, scale, grass)` — return the
/// pre-1.18 biome metadata triple, or `None` if the biome ID is
/// unknown. Bit-exact port of cubiomes' switch in `biomenoise.c`.
#[must_use]
#[allow(clippy::too_many_lines, clippy::match_same_arms)]
pub fn get_biome_depth_and_scale(id: i32) -> Option<BiomeDepthScale> {
    // dh = default height (62).
    const DH: i32 = 62;
    let (s, d, g) = match id {
        0 => (0.100, -1.000, DH),   // ocean
        1 => (0.050, 0.125, DH),    // plains
        2 => (0.050, 0.125, 0),     // desert
        3 => (0.500, 1.000, DH),    // mountains
        4 => (0.200, 0.100, DH),    // forest
        5 => (0.200, 0.200, DH),    // taiga
        6 => (0.100, -0.200, DH),   // swamp
        7 => (0.000, -0.500, 60),   // river
        10 => (0.100, -1.000, DH),  // frozen_ocean
        11 => (0.000, -0.500, 60),  // frozen_river
        12 => (0.050, 0.125, DH),   // snowy_tundra
        13 => (0.300, 0.450, DH),   // snowy_mountains
        14 => (0.300, 0.200, 0),    // mushroom_fields
        15 => (0.025, 0.000, 0),    // mushroom_field_shore
        16 => (0.025, 0.000, 64),   // beach
        17 => (0.300, 0.450, 0),    // desert_hills
        18 => (0.300, 0.450, DH),   // wooded_hills
        19 => (0.300, 0.450, DH),   // taiga_hills
        20 => (0.300, 0.800, DH),   // mountain_edge
        21 => (0.200, 0.100, DH),   // jungle
        22 => (0.300, 0.450, DH),   // jungle_hills
        23 => (0.200, 0.100, DH),   // jungle_edge
        24 => (0.100, -1.800, DH),  // deep_ocean
        25 => (0.800, 0.100, 64),   // stone_shore
        26 => (0.025, 0.000, 64),   // snowy_beach
        27 => (0.200, 0.100, DH),   // birch_forest
        28 => (0.300, 0.450, DH),   // birch_forest_hills
        29 => (0.200, 0.100, DH),   // dark_forest
        30 => (0.200, 0.200, DH),   // snowy_taiga
        31 => (0.300, 0.450, DH),   // snowy_taiga_hills
        32 => (0.200, 0.200, DH),   // giant_tree_taiga
        33 => (0.300, 0.450, DH),   // giant_tree_taiga_hills
        34 => (0.500, 1.000, DH),   // wooded_mountains
        35 => (0.050, 0.125, DH),   // savanna
        36 => (0.025, 1.500, DH),   // savanna_plateau
        37 => (0.200, 0.100, 0),    // badlands
        38 => (0.025, 1.500, 0),    // wooded_badlands_plateau
        39 => (0.025, 1.500, 0),    // badlands_plateau
        44 => (0.100, -1.000, 0),   // warm_ocean
        45 => (0.100, -1.000, DH),  // lukewarm_ocean
        46 => (0.100, -1.000, DH),  // cold_ocean
        47 => (0.100, -1.800, 0),   // deep_warm_ocean
        48 => (0.100, -1.800, DH),  // deep_lukewarm_ocean
        49 => (0.100, -1.800, DH),  // deep_cold_ocean
        50 => (0.100, -1.800, DH),  // deep_frozen_ocean
        129 => (0.050, 0.125, DH),  // sunflower_plains
        130 => (0.250, 0.225, 0),   // desert_lakes
        131 => (0.500, 1.000, DH),  // gravelly_mountains
        132 => (0.400, 0.100, DH),  // flower_forest
        133 => (0.400, 0.300, DH),  // taiga_mountains
        134 => (0.300, -0.100, DH), // swamp_hills
        140 => (0.450, 0.425, 0),   // ice_spikes
        149 => (0.400, 0.200, DH),  // modified_jungle
        151 => (0.400, 0.200, DH),  // modified_jungle_edge
        155 => (0.400, 0.200, DH),  // tall_birch_forest
        156 => (0.500, 0.550, DH),  // tall_birch_hills
        157 => (0.400, 0.200, DH),  // dark_forest_hills
        158 => (0.400, 0.300, DH),  // snowy_taiga_mountains
        160 => (0.200, 0.200, DH),  // giant_spruce_taiga
        161 => (0.200, 0.200, DH),  // giant_spruce_taiga_hills
        162 => (0.500, 1.000, DH),  // modified_gravelly_mountains
        163 => (1.225, 0.3625, DH), // shattered_savanna
        164 => (1.212, 1.050, DH),  // shattered_savanna_plateau
        165 => (0.200, 0.100, 0),   // eroded_badlands
        166 => (0.300, 0.450, 0),   // modified_wooded_badlands_plateau
        167 => (0.300, 0.450, 0),   // modified_badlands_plateau
        168 => (0.200, 0.100, DH),  // bamboo_jungle
        169 => (0.300, 0.450, DH),  // bamboo_jungle_hills
        _ => return None,
    };
    Some(BiomeDepthScale {
        depth: d,
        scale: s,
        grass: g,
    })
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
