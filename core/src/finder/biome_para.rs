//! Climate-parameter bounds for 1.18+ biome noise.
//!
//! Ports cubiomes' `getBiomeParaExtremes` from `finders.c`. Returns
//! the min/max integer bounds along each climate axis (temperature,
//! humidity, continentalness, erosion, depth, weirdness) for the
//! given MC version.

use crate::mc_version::MCVersion;

/// One min/max pair on a climate axis (integer-quantised, like
/// cubiomes' `int` table entries).
pub type ParaRange = (i32, i32);

/// Six climate axes: T, H, C, E, D, W.
pub type ParaExtremes = [ParaRange; 6];

const BETA_EXTREMES: ParaExtremes = [(0, 10000), (0, 10000), (0, 0), (0, 0), (0, 0), (0, 0)];

const MODERN_EXTREMES: ParaExtremes = [
    (-4501, 5500),
    (-3500, 6999),
    (-10500, 300),
    (-7799, 5500),
    (1000, 10500),
    (-9333, 9333),
];

/// Returns the min/max parameter values within which a biome change
/// can occur, or `None` for versions that don't expose climate
/// parameters (MC 1.0–1.17 layered worlds).
#[must_use]
pub fn get_biome_para_extremes(mc: MCVersion) -> Option<ParaExtremes> {
    // mc <= MC_B1_7
    if !mc.is_at_least(MCVersion::B1_8) {
        return Some(BETA_EXTREMES);
    }
    // mc <= MC_1_17
    if !mc.is_at_least(MCVersion::V1_18) {
        return None;
    }
    Some(MODERN_EXTREMES)
}

const IMIN: i32 = i32::MIN;
const IMAX: i32 = i32::MAX;

type ParaRow = (i32, ParaExtremes);

/// 1.18 base table — `g_biome_para_range_18` in cubiomes `finders.c`.
#[rustfmt::skip]
const PARA_RANGE_18: &[ParaRow] = &[
    (0,   [(-1500, 2000), (IMIN, IMAX), (-4550,-1900), (IMIN, IMAX), (IMIN, IMAX), (IMIN, IMAX)]), // ocean
    (1,   [(-4500, 5500), (IMIN, 1000), (-1900, IMAX), (IMIN, IMAX), (IMIN, IMAX), (IMIN, IMAX)]), // plains
    (2,   [(5500,  IMAX), (IMIN, IMAX), (-1900, IMAX), (IMIN, IMAX), (IMIN, IMAX), (IMIN, IMAX)]), // desert
    (3,   [(IMIN,  2000), (IMIN, 1000), (-1899, IMAX), (4500, 5500), (IMIN, IMAX), (IMIN, IMAX)]), // windswept_hills
    (4,   [(-4500, 5500), (-1000,3000), (-1900, IMAX), (IMIN, IMAX), (IMIN, IMAX), (IMIN, IMAX)]), // forest
    (5,   [(IMIN, -1500), (1000, IMAX), (-1900, IMAX), (IMIN, IMAX), (IMIN, IMAX), (IMIN, IMAX)]), // taiga
    (6,   [(-4500, IMAX), (IMIN, IMAX), (-1100, IMAX), (5500, IMAX), (IMIN, IMAX), (IMIN, IMAX)]), // swamp
    (7,   [(-4500, IMAX), (IMIN, IMAX), (-1900, IMAX), (IMIN, IMAX), (IMIN, IMAX), (-500,  500)]), // river
    (10,  [(IMIN, -4501), (IMIN, IMAX), (-4550,-1900), (IMIN, IMAX), (IMIN, IMAX), (IMIN, IMAX)]), // frozen_ocean
    (11,  [(IMIN, -4501), (IMIN, IMAX), (-1900, IMAX), (IMIN, IMAX), (IMIN, IMAX), (-500,  500)]), // frozen_river
    (12,  [(IMIN, -4500), (IMIN, 1000), (-1900, IMAX), (IMIN, IMAX), (IMIN, IMAX), (IMIN, IMAX)]), // snowy_plains
    (14,  [(IMIN, IMAX),  (IMIN, IMAX), (IMIN,-10500), (IMIN, IMAX), (IMIN, IMAX), (IMIN, IMAX)]), // mushroom_fields
    (16,  [(-4500, 5500), (IMIN, IMAX), (-1900,-1100), (-2225, IMAX),(IMIN, IMAX), (IMIN, 2666)]), // beach
    (21,  [(2000,  5500), (1000, IMAX), (-1900, IMAX), (IMIN, IMAX), (IMIN, IMAX), (IMIN, IMAX)]), // jungle
    (23,  [(2000,  5500), (1000, 3000), (-1899, IMAX), (IMIN, IMAX), (IMIN, IMAX), (-500,  IMAX)]), // sparse_jungle
    (24,  [(-1500, 2000), (IMIN, IMAX), (-10500,-4551),(IMIN, IMAX), (IMIN, IMAX), (IMIN, IMAX)]), // deep_ocean
    (25,  [(IMIN,  IMAX), (IMIN, IMAX), (-1900,-1100), (IMIN, -2225),(IMIN, IMAX), (IMIN, IMAX)]), // stony_shore
    (26,  [(IMIN, -4500), (IMIN, IMAX), (-1900,-1100), (-2225, IMAX),(IMIN, IMAX), (IMIN, 2666)]), // snowy_beach
    (27,  [(-1500, 2000), (1000, 3000), (-1900, IMAX), (IMIN, IMAX), (IMIN, IMAX), (IMIN, IMAX)]), // birch_forest
    (29,  [(-1500, 2000), (3000, IMAX), (-1900, IMAX), (IMIN, IMAX), (IMIN, IMAX), (IMIN, IMAX)]), // dark_forest
    (30,  [(IMIN, -4500), (-1000,IMAX), (-1900, IMAX), (IMIN, IMAX), (IMIN, IMAX), (IMIN, IMAX)]), // snowy_taiga
    (32,  [(-4500,-1500), (3000, IMAX), (-1899, IMAX), (IMIN, IMAX), (IMIN, IMAX), (-500,  IMAX)]), // old_growth_pine_taiga
    (34,  [(IMIN,  2000), (1000, IMAX), (-1899, IMAX), (4500, 5500), (IMIN, IMAX), (IMIN, IMAX)]), // windswept_forest
    (35,  [(2000,  5500), (IMIN,-1000), (-1900, IMAX), (IMIN, IMAX), (IMIN, IMAX), (IMIN, IMAX)]), // savanna
    (36,  [(2000,  5500), (IMIN,-1000), (-1100, IMAX), (IMIN, 500),  (IMIN, IMAX), (IMIN, IMAX)]), // savanna_plateau
    (37,  [(5500,  IMAX), (IMIN, 1000), (-1899, IMAX), (IMIN, 500),  (IMIN, IMAX), (IMIN, IMAX)]), // badlands
    (38,  [(5500,  IMAX), (1000, IMAX), (-1899, IMAX), (IMIN, 500),  (IMIN, IMAX), (IMIN, IMAX)]), // wooded_badlands
    (44,  [(5500,  IMAX), (IMIN, IMAX), (-10500,-1900),(IMIN, IMAX), (IMIN, IMAX), (IMIN, IMAX)]), // warm_ocean
    (45,  [(2001,  5500), (IMIN, IMAX), (-4550,-1900), (IMIN, IMAX), (IMIN, IMAX), (IMIN, IMAX)]), // lukewarm_ocean
    (46,  [(-4500,-1501), (IMIN, IMAX), (-4550,-1900), (IMIN, IMAX), (IMIN, IMAX), (IMIN, IMAX)]), // cold_ocean
    (48,  [(2001,  5500), (IMIN, IMAX), (-10500,-4551),(IMIN, IMAX), (IMIN, IMAX), (IMIN, IMAX)]), // deep_lukewarm_ocean
    (49,  [(-4500,-1501), (IMIN, IMAX), (-10500,-4551),(IMIN, IMAX), (IMIN, IMAX), (IMIN, IMAX)]), // deep_cold_ocean
    (50,  [(IMIN, -4501), (IMIN, IMAX), (-10500,-4551),(IMIN, IMAX), (IMIN, IMAX), (IMIN, IMAX)]), // deep_frozen_ocean
    (129, [(-1500, 2000), (IMIN,-3500), (-1899, IMAX), (IMIN, IMAX), (IMIN, IMAX), (-500,  IMAX)]), // sunflower_plains
    (131, [(IMIN, -1500), (IMIN,-1000), (-1899, IMAX), (4500, 5500), (IMIN, IMAX), (IMIN, IMAX)]), // windswept_gravelly_hills
    (132, [(-1500, 2000), (IMIN,-3500), (-1900, IMAX), (IMIN, IMAX), (IMIN, IMAX), (IMIN, -500)]), // flower_forest
    (140, [(IMIN, -4500), (IMIN,-3500), (-1899, IMAX), (IMIN, IMAX), (IMIN, IMAX), (-500,  IMAX)]), // ice_spikes
    (155, [(-1500, 2000), (1000, 3000), (-1899, IMAX), (IMIN, IMAX), (IMIN, IMAX), (-500,  IMAX)]), // old_growth_birch_forest
    (160, [(-4500,-1500), (3000, IMAX), (-1900, IMAX), (IMIN, IMAX), (IMIN, IMAX), (IMIN, -500)]), // old_growth_spruce_taiga
    (163, [(-1500, IMAX), (IMIN, 3000), (-1899, 300),  (4500, 5500), (IMIN, IMAX), (501,   IMAX)]), // windswept_savanna
    (165, [(5500,  IMAX), (IMIN,-1000), (-1899, IMAX), (IMIN, 500),  (IMIN, IMAX), (IMIN, IMAX)]), // eroded_badlands
    (168, [(2000,  5500), (3000, IMAX), (-1899, IMAX), (IMIN, IMAX), (IMIN, IMAX), (-500,  IMAX)]), // bamboo_jungle
    (174, [(IMIN,  IMAX), (IMIN, 6999), (3001,  IMAX), (IMIN, IMAX), (1000, 9500), (IMIN, IMAX)]), // dripstone_caves
    (175, [(IMIN,  IMAX), (2001, IMAX), (IMIN,  IMAX), (IMIN, IMAX), (1000, 9500), (IMIN, IMAX)]), // lush_caves
    (177, [(-4500, 2000), (IMIN, 3000), (300,   IMAX), (-7799, 500), (IMIN, IMAX), (IMIN, IMAX)]), // meadow
    (178, [(IMIN,  2000), (-1000,IMAX), (-1899, IMAX), (IMIN,-3750), (IMIN, IMAX), (IMIN, IMAX)]), // grove
    (179, [(IMIN,  2000), (IMIN,-1000), (-1899, IMAX), (IMIN,-3750), (IMIN, IMAX), (IMIN, IMAX)]), // snowy_slopes
    (180, [(IMIN,  2000), (IMIN, IMAX), (-1899, IMAX), (IMIN,-3750), (IMIN, IMAX), (-9333,-4001)]), // jagged_peaks
    (181, [(IMIN,  2000), (IMIN, IMAX), (-1899, IMAX), (IMIN,-3750), (IMIN, IMAX), (4000,  9333)]), // frozen_peaks
    (182, [(2000,  5500), (IMIN, IMAX), (-1899, IMAX), (IMIN,-3750), (IMIN, IMAX), (-9333, 9333)]), // stony_peaks
];

/// 1.19 diff — `g_biome_para_range_19_diff`.
#[rustfmt::skip]
const PARA_RANGE_19_DIFF: &[ParaRow] = &[
    (165, [(5500,  IMAX), (IMIN,-1000), (-1899, IMAX), (IMIN,  500), (IMIN, IMAX), (-500,  IMAX)]), // eroded_badlands
    (178, [(IMIN,  2000), (-1000,IMAX), (-1899, IMAX), (IMIN,-3750), (IMIN,10499), (IMIN, IMAX)]), // grove
    (179, [(IMIN,  2000), (IMIN,-1000), (-1899, IMAX), (IMIN,-3750), (IMIN,10499), (IMIN, IMAX)]), // snowy_slopes
    (180, [(IMIN,  2000), (IMIN, IMAX), (-1899, IMAX), (IMIN,-3750), (IMIN,10499), (-9333,-4001)]), // jagged_peaks
    (183, [(IMIN,  IMAX), (IMIN, IMAX), (IMIN,  IMAX), (IMIN, 1818), (10500,IMAX), (IMIN, IMAX)]), // deep_dark
    (184, [(2000,  IMAX), (IMIN, IMAX), (-1100, IMAX), (5500, IMAX), (IMIN, IMAX), (IMIN, IMAX)]), // mangrove_swamp
];

/// 1.20 diff — `g_biome_para_range_20_diff`.
#[rustfmt::skip]
const PARA_RANGE_20_DIFF: &[ParaRow] = &[
    (6,   [(-4500, 2000), (IMIN, IMAX), (-1100, IMAX), (5500, IMAX), (IMIN, IMAX), (IMIN, IMAX)]), // swamp
    (178, [(IMIN,  2000), (-1000,IMAX), (-1899, IMAX), (IMIN,-3750), (IMIN,10500), (IMIN, IMAX)]), // grove
    (179, [(IMIN,  2000), (IMIN,-1000), (-1899, IMAX), (IMIN,-3750), (IMIN,10500), (IMIN, IMAX)]), // snowy_slopes
    (180, [(IMIN,  2000), (IMIN, IMAX), (-1899, IMAX), (IMIN,-3750), (IMIN,10500), (-9333,-4000)]), // jagged_peaks
    (181, [(IMIN,  2000), (IMIN, IMAX), (-1899, IMAX), (IMIN,-3750), (IMIN,10500), (4000,  9333)]), // frozen_peaks
    (182, [(2000,  5500), (IMIN, IMAX), (-1899, IMAX), (IMIN,-3750), (IMIN,10500), (-9333, 9333)]), // stony_peaks
    (185, [(-4500, 2000), (IMIN,-1000), (300,   IMAX), (-7799, 500), (IMIN, IMAX), (2666,  IMAX)]), // cherry_grove
];

/// 1.21 winter-drop diff — `g_biome_para_range_21wd_diff`.
#[rustfmt::skip]
const PARA_RANGE_21WD_DIFF: &[ParaRow] = &[
    (186, [(-1500, 2000), (3000, IMAX), (300,   IMAX), (-7799, 500), (IMIN, IMAX), (2666,  IMAX)]), // pale_garden
];

fn lookup(table: &[ParaRow], id: i32) -> Option<ParaExtremes> {
    for row in table {
        if row.0 == id {
            return Some(row.1);
        }
    }
    None
}

/// Returns the min/max climate-parameter bounds for `id` at the
/// given MC version, or `None` if `id` does not generate (or
/// version is layered).
///
/// Cubiomes parity: replays the `_21wd` → `_20` → `_19` → `_18`
/// fallthrough order from `getBiomeParaLimits`, so the first match
/// wins.
#[must_use]
pub fn get_biome_para_limits(mc: MCVersion, id: i32) -> Option<ParaExtremes> {
    if !mc.is_at_least(MCVersion::V1_18) {
        return None;
    }
    if mc.is_at_least(MCVersion::V1_21) {
        if let Some(r) = lookup(PARA_RANGE_21WD_DIFF, id) {
            return Some(r);
        }
    }
    if mc.is_at_least(MCVersion::V1_20) {
        if let Some(r) = lookup(PARA_RANGE_20_DIFF, id) {
            return Some(r);
        }
    }
    if mc.is_at_least(MCVersion::V1_19_2) {
        if let Some(r) = lookup(PARA_RANGE_19_DIFF, id) {
            return Some(r);
        }
    }
    lookup(PARA_RANGE_18, id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beta_returns_beta_extremes() {
        let r = get_biome_para_extremes(MCVersion::B1_7).expect("beta has extremes");
        assert_eq!(r[0], (0, 10000));
        assert_eq!(r[1], (0, 10000));
        assert_eq!(r[2], (0, 0));
    }

    #[test]
    fn layered_returns_none() {
        assert!(get_biome_para_extremes(MCVersion::V1_17).is_none());
        assert!(get_biome_para_extremes(MCVersion::V1_13).is_none());
    }

    #[test]
    fn modern_returns_climate_bounds() {
        let r = get_biome_para_extremes(MCVersion::V1_18).expect("modern has extremes");
        assert_eq!(r[0], (-4501, 5500));
        assert_eq!(r[5], (-9333, 9333));
    }
}
