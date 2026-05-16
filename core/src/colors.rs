//! Biome → RGB color tables for visualisation. Bit-exact port of
//! cubiomes' `initBiomeColors` / `initBiomeTypeColors` from `util.c`.
//!
//! Color scheme inspired by the AMIDST project
//! (<https://github.com/toolbox4minecraft/amidst/wiki/Biome-Color-Table>)
//! with cubiomes' additions for 1.18+ and a few contrast tweaks.

#![allow(clippy::missing_panics_doc, clippy::unreadable_literal)]

/// Build the AMIDST-style 256-entry biome RGB color table. Biomes
/// not present in cubiomes' table return `[0, 0, 0]` (matching the
/// `memset(0)` initialisation in cubiomes).
///
/// Bit-exact port of cubiomes' `initBiomeColors`.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn init_biome_colors() -> [[u8; 3]; 256] {
    let mut colors = [[0u8; 3]; 256];
    let entries: &[(usize, u32)] = &[
        (0, 0x000070),   // ocean
        (1, 0x8db360),   // plains
        (2, 0xfa9418),   // desert
        (3, 0x606060),   // mountains / windswept_hills
        (4, 0x056621),   // forest
        (5, 0x0b6a5f),   // taiga
        (6, 0x07f9b2),   // swamp
        (7, 0x0000ff),   // river
        (8, 0x572526),   // nether_wastes
        (9, 0x8080ff),   // the_end
        (10, 0x7070d6),  // frozen_ocean
        (11, 0xa0a0ff),  // frozen_river
        (12, 0xffffff),  // snowy_plains / snowy_tundra
        (13, 0xa0a0a0),  // snowy_mountains
        (14, 0xff00ff),  // mushroom_fields
        (15, 0xa000ff),  // mushroom_field_shore
        (16, 0xfade55),  // beach
        (17, 0xd25f12),  // desert_hills
        (18, 0x22551c),  // wooded_hills
        (19, 0x163933),  // taiga_hills
        (20, 0x72789a),  // mountain_edge
        (21, 0x507b0a),  // jungle
        (22, 0x2c4205),  // jungle_hills
        (23, 0x60930f),  // sparse_jungle / jungle_edge
        (24, 0x000030),  // deep_ocean
        (25, 0xa2a284),  // stony_shore / stone_shore
        (26, 0xfaf0c0),  // snowy_beach
        (27, 0x307444),  // birch_forest
        (28, 0x1f5f32),  // birch_forest_hills
        (29, 0x40511a),  // dark_forest
        (30, 0x31554a),  // snowy_taiga
        (31, 0x243f36),  // snowy_taiga_hills
        (32, 0x596651),  // old_growth_pine_taiga / giant_tree_taiga
        (33, 0x454f3e),  // giant_tree_taiga_hills
        (34, 0x5b7352),  // windswept_forest / wooded_mountains
        (35, 0xbdb25f),  // savanna
        (36, 0xa79d64),  // savanna_plateau
        (37, 0xd94515),  // badlands
        (38, 0xb09765),  // wooded_badlands / wooded_badlands_plateau
        (39, 0xca8c65),  // badlands_plateau
        (40, 0x4b4bab),  // small_end_islands
        (41, 0xc9c959),  // end_midlands
        (42, 0xb5b536),  // end_highlands
        (43, 0x7070cc),  // end_barrens
        (44, 0x0000ac),  // warm_ocean
        (45, 0x000090),  // lukewarm_ocean
        (46, 0x202070),  // cold_ocean
        (47, 0x000050),  // deep_warm_ocean
        (48, 0x000040),  // deep_lukewarm_ocean
        (49, 0x202038),  // deep_cold_ocean
        (50, 0x404090),  // deep_frozen_ocean
        (51, 0x2f560f),  // seasonal_forest
        (52, 0x47840e),  // rainforest
        (53, 0x789e31),  // shrubland
        (127, 0x000000), // the_void
        (129, 0xb5db88), // sunflower_plains
        (130, 0xffbc40), // desert_lakes
        (131, 0x888888), // windswept_gravelly_hills / gravelly_mountains
        (132, 0x2d8e49), // flower_forest
        (133, 0x339287), // taiga_mountains
        (134, 0x2fffda), // swamp_hills
        (140, 0xb4dcdc), // ice_spikes
        (149, 0x78a332), // modified_jungle
        (151, 0x88bb37), // modified_jungle_edge
        (155, 0x589c6c), // old_growth_birch_forest / tall_birch_forest
        (156, 0x47875a), // tall_birch_hills
        (157, 0x687942), // dark_forest_hills
        (158, 0x597d72), // snowy_taiga_mountains
        (160, 0x818e79), // old_growth_spruce_taiga / giant_spruce_taiga
        (161, 0x6d7766), // giant_spruce_taiga_hills
        (162, 0x839b7a), // modified_gravelly_mountains
        (163, 0xe5da87), // windswept_savanna / shattered_savanna
        (164, 0xcfc58c), // shattered_savanna_plateau
        (165, 0xff6d3d), // eroded_badlands
        (166, 0xd8bf8d), // modified_wooded_badlands_plateau
        (167, 0xf2b48d), // modified_badlands_plateau
        (168, 0x849500), // bamboo_jungle
        (169, 0x5c6c04), // bamboo_jungle_hills
        (170, 0x4d3a2e), // soul_sand_valley
        (171, 0x981a11), // crimson_forest
        (172, 0x49907b), // warped_forest
        (173, 0x645f63), // basalt_deltas
        (174, 0x4e3012), // dripstone_caves
        (175, 0x283c00), // lush_caves
        (177, 0x60a445), // meadow
        (178, 0x47726c), // grove
        (179, 0xc4c4c4), // snowy_slopes
        (180, 0xdcdcc8), // jagged_peaks
        (181, 0xb0b3ce), // frozen_peaks
        (182, 0x7b8f74), // stony_peaks
        (183, 0x031f29), // deep_dark
        (184, 0x2ccc8e), // mangrove_swamp
        (185, 0xff91c8), // cherry_grove
        (186, 0x696d95), // pale_garden
    ];
    for &(id, hex) in entries {
        colors[id] = [
            ((hex >> 16) & 0xff) as u8,
            ((hex >> 8) & 0xff) as u8,
            (hex & 0xff) as u8,
        ];
    }
    colors
}

/// `initBiomeTypeColors` — 5-entry palette keyed by the climate
/// category (`Oceanic`, `Warm`, `Lush`, `Cold`, `Freezing`).
/// Mirrors cubiomes' helper of the same name.
#[must_use]
pub fn init_biome_type_colors() -> [[u8; 3]; 256] {
    let mut colors = [[0u8; 3]; 256];
    let entries: &[(usize, u32)] = &[
        // cubiomes' BiomeTempCategory enum maps to small ids.
        (0, 0x0000a0), // Oceanic
        (1, 0xffc000), // Warm
        (2, 0x00a000), // Lush
        (3, 0x606060), // Cold
        (4, 0xffffff), // Freezing
    ];
    for &(id, hex) in entries {
        colors[id] = [
            ((hex >> 16) & 0xff) as u8,
            ((hex >> 8) & 0xff) as u8,
            (hex & 0xff) as u8,
        ];
    }
    colors
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ocean_color_matches_amidst() {
        let c = init_biome_colors();
        assert_eq!(c[0], [0x00, 0x00, 0x70]);
    }

    #[test]
    fn high_ids_default_to_black() {
        let c = init_biome_colors();
        assert_eq!(c[200], [0, 0, 0]);
        assert_eq!(c[255], [0, 0, 0]);
    }

    #[test]
    fn pale_garden_present() {
        let c = init_biome_colors();
        assert_eq!(c[186], [0x69, 0x6d, 0x95]);
    }

    #[test]
    fn biome_type_palette_size() {
        let c = init_biome_type_colors();
        // First 5 entries set, rest zero.
        assert_eq!(c[0], [0x00, 0x00, 0xa0]);
        assert_eq!(c[4], [0xff, 0xff, 0xff]);
        assert_eq!(c[5], [0, 0, 0]);
    }
}
