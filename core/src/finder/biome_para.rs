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
