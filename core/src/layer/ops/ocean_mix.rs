//! `mapOceanMix` — combine the biome chain output with the ocean
//! temperature chain output, producing the final ocean variants
//! (warm / lukewarm / ocean / cold / frozen, plus their deep
//! counterparts).
//!
//! Bit-exact port of cubiomes' `mapOceanMix`. The land parent is
//! supplied as a contiguous slice whose bounding box was sized to
//! exactly cover the cells `mapOceanMix` ever touches — see
//! [`ocean_land_bbox`]: it scans the ocean grid for `WARM_OCEAN` /
//! `FROZEN_OCEAN` cells and expands `(lx0, lx1, lz0, lz1)` so that any
//! warm/frozen cell's 17×17 neighbourhood (`-8..=8` inclusive) is
//! always inside the slice. Cells with no warm/frozen ocean only
//! consult their own centre, so the default extent of `(0, 0, w, h)`
//! is enough when no warm/frozen ocean is present.

#![allow(clippy::many_single_char_names)]
#![allow(clippy::too_many_arguments)]

use crate::biome::Biome;

const OCEAN: i32 = Biome::OCEAN.id();
const WARM_OCEAN: i32 = Biome::WARM_OCEAN.id();
const LUKEWARM_OCEAN: i32 = Biome::LUKEWARM_OCEAN.id();
const COLD_OCEAN: i32 = Biome::COLD_OCEAN.id();
const FROZEN_OCEAN: i32 = Biome::FROZEN_OCEAN.id();
const DEEP_OCEAN: i32 = Biome::DEEP_OCEAN.id();
const DEEP_LUKEWARM_OCEAN: i32 = Biome::DEEP_LUKEWARM_OCEAN.id();
const DEEP_COLD_OCEAN: i32 = Biome::DEEP_COLD_OCEAN.id();
const DEEP_FROZEN_OCEAN: i32 = Biome::DEEP_FROZEN_OCEAN.id();

/// Minimum bounding box of the land parent required by
/// [`map_ocean_mix`], expressed as offsets relative to the ocean grid
/// `(0, 0)..(w, h)`. The returned `(lx0, lx1, lz0, lz1)` satisfies
/// `lx0 <= 0`, `lx1 >= w`, `lz0 <= 0`, `lz1 >= h`. Width / height of
/// the land slice are `lx1 - lx0` / `lz1 - lz0`.
pub fn ocean_land_bbox(ocean: &[Biome], w: usize, h: usize) -> (i32, i32, i32, i32) {
    let w_i = w as i32;
    let h_i = h as i32;
    let mut lx0: i32 = 0;
    let mut lx1: i32 = w_i;
    let mut lz0: i32 = 0;
    let mut lz1: i32 = h_i;
    for j in 0..h_i {
        let jcentre = j - 8 > 0 && j + 9 < h_i;
        for i in 0..w_i {
            if jcentre && i - 8 > 0 && i + 9 < w_i {
                continue;
            }
            let ocean_id = ocean[(i + j * w_i) as usize].id();
            if ocean_id == WARM_OCEAN || ocean_id == FROZEN_OCEAN {
                if i - 8 < lx0 {
                    lx0 = i - 8;
                }
                if i + 9 > lx1 {
                    lx1 = i + 9;
                }
                if j - 8 < lz0 {
                    lz0 = j - 8;
                }
                if j + 9 > lz1 {
                    lz1 = j + 9;
                }
            }
        }
    }
    (lx0, lx1, lz0, lz1)
}

/// `mapOceanMix` — combine the ocean temperature grid with the land
/// biome grid.
///
/// - `ocean`: ocean temperature output, `w * h` cells in row-major
///   order, addressing `(x + i, z + j)`.
/// - `land`: land biome chain output, `lw * lh` cells. Its origin in
///   ocean coordinates is `(lx0, lz0)`: `land[ii + jj * lw]` covers
///   ocean cell `(lx0 + ii, lz0 + jj)`. `(lx0, lz0, lw, lh)` must
///   contain every cell `mapOceanMix` reads, which for any cell with
///   no warm/frozen ocean is just the centre, and for warm/frozen
///   ocean cells is a 17×17 neighbourhood. [`ocean_land_bbox`]
///   returns the minimum bounding box.
/// - `out`: target slice, `w * h` cells. May alias `ocean` if the
///   caller wants cubiomes' in-place behaviour, but the borrow checker
///   forces the caller to copy in that case.
pub fn map_ocean_mix(
    ocean: &[Biome],
    land: &[Biome],
    out: &mut [Biome],
    w: usize,
    h: usize,
    lx0: i32,
    lz0: i32,
    lw: usize,
    lh: usize,
) {
    assert!(ocean.len() >= w * h, "map_ocean_mix: ocean slice too small");
    assert!(land.len() >= lw * lh, "map_ocean_mix: land slice too small");
    assert!(out.len() >= w * h, "map_ocean_mix: output slice too small");
    let lw_i = lw as i32;
    let lh_i = lh as i32;
    let w_i = w as i32;
    let h_i = h as i32;
    assert!(
        -lx0 + w_i <= lw_i && -lz0 + h_i <= lh_i && lx0 <= 0 && lz0 <= 0,
        "map_ocean_mix: land bounding box does not contain the ocean grid"
    );

    for j in 0..h {
        for i in 0..w {
            let oi = i as i32;
            let oj = j as i32;
            let land_id = land[((oi - lx0) + (oj - lz0) * lw_i) as usize].id();
            let ocean_id = ocean[i + j * w].id();

            if !Biome::is_oceanic_id(land_id) {
                out[i + j * w] = Biome(land_id);
                continue;
            }

            let replace_id = if ocean_id == WARM_OCEAN {
                LUKEWARM_OCEAN
            } else if ocean_id == FROZEN_OCEAN {
                COLD_OCEAN
            } else {
                0
            };

            if replace_id != 0 && nearby_has_land(land, lw_i, lh_i, oi - lx0, oj - lz0) {
                out[i + j * w] = Biome(replace_id);
                continue;
            }

            let result = if land_id == DEEP_OCEAN {
                match ocean_id {
                    LUKEWARM_OCEAN => DEEP_LUKEWARM_OCEAN,
                    OCEAN => DEEP_OCEAN,
                    COLD_OCEAN => DEEP_COLD_OCEAN,
                    FROZEN_OCEAN => DEEP_FROZEN_OCEAN,
                    _ => ocean_id,
                }
            } else {
                ocean_id
            };
            out[i + j * w] = Biome(result);
        }
    }
}

/// `mapOceanMixMod` — used when the [`Generator`] is constructed
/// with the `FORCE_OCEAN_VARIANTS` flag.
///
/// Takes the same `(ocean, land)` pair as [`map_ocean_mix`] but
/// applies a simpler rule: every oceanic cell in `land` is replaced
/// with the temperature variant from `ocean`, with `DEEP_OCEAN`
/// promoted to `DEEP_LUKEWARM_OCEAN` / `DEEP_OCEAN` /
/// `DEEP_COLD_OCEAN` / `DEEP_FROZEN_OCEAN` based on the ocean type.
/// Non-oceanic cells are passed through unchanged.
///
/// Unlike [`map_ocean_mix`], the land and ocean slices are the same
/// `w × h` size — no nearby-land 17×17 dilation.
///
/// Bit-exact port of `cubiomes/generator.c::mapOceanMixMod`.
///
/// [`Generator`]: crate::generator::Generator
pub fn map_ocean_mix_mod(ocean: &[Biome], land: &[Biome], out: &mut [Biome], w: usize, h: usize) {
    assert!(ocean.len() >= w * h, "map_ocean_mix_mod: ocean too small");
    assert!(land.len() >= w * h, "map_ocean_mix_mod: land too small");
    assert!(out.len() >= w * h, "map_ocean_mix_mod: out too small");

    for j in 0..h {
        for i in 0..w {
            let land_id = land[i + j * w].id();
            if !Biome::is_oceanic_id(land_id) {
                out[i + j * w] = Biome(land_id);
                continue;
            }
            let ocean_id = ocean[i + j * w].id();
            let result = if land_id == DEEP_OCEAN {
                match ocean_id {
                    LUKEWARM_OCEAN => DEEP_LUKEWARM_OCEAN,
                    OCEAN => DEEP_OCEAN,
                    COLD_OCEAN => DEEP_COLD_OCEAN,
                    FROZEN_OCEAN => DEEP_FROZEN_OCEAN,
                    _ => ocean_id,
                }
            } else {
                ocean_id
            };
            out[i + j * w] = Biome(result);
        }
    }
}

/// Returns `true` iff at least one biome cell within the 17×17 grid
/// centred on `(centre_x, centre_z)` (step 4, `ii / jj in -8..=8`) is
/// non-oceanic. `centre_x` / `centre_z` are land-slice coordinates.
fn nearby_has_land(land: &[Biome], lw: i32, lh: i32, centre_x: i32, centre_z: i32) -> bool {
    let mut jj: i32 = -8;
    while jj <= 8 {
        let z = centre_z + jj;
        if z >= 0 && z < lh {
            let mut ii: i32 = -8;
            while ii <= 8 {
                let x = centre_x + ii;
                if x >= 0 && x < lw {
                    let id = land[(x + z * lw) as usize].id();
                    if !Biome::is_oceanic_id(id) {
                        return true;
                    }
                }
                ii += 4;
            }
        }
        jj += 4;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uniform_land(id: i32, lw: usize, lh: usize) -> Vec<Biome> {
        vec![Biome(id); lw * lh]
    }

    #[test]
    fn no_warm_or_frozen_keeps_default_bbox() {
        let ocean = vec![Biome::OCEAN; 8 * 8];
        let bbox = ocean_land_bbox(&ocean, 8, 8);
        assert_eq!(bbox, (0, 8, 0, 8));
    }

    #[test]
    fn warm_ocean_at_origin_expands_bbox() {
        let mut ocean = vec![Biome::OCEAN; 4 * 4];
        ocean[0] = Biome::WARM_OCEAN;
        let bbox = ocean_land_bbox(&ocean, 4, 4);
        // i=0, j=0: lx0 ← min(0, 0-8) = -8, lx1 ← max(4, 0+9) = 9,
        //          lz0 ← min(0, 0-8) = -8, lz1 ← max(4, 0+9) = 9.
        assert_eq!(bbox, (-8, 9, -8, 9));
    }

    #[test]
    fn full_ocean_input_passes_ocean_through() {
        let ocean = vec![Biome::OCEAN; 4 * 4];
        let land = uniform_land(Biome::OCEAN.id(), 4, 4);
        let mut out = vec![Biome::NONE; 4 * 4];
        map_ocean_mix(&ocean, &land, &mut out, 4, 4, 0, 0, 4, 4);
        for cell in &out {
            assert_eq!(*cell, Biome::OCEAN);
        }
    }

    #[test]
    fn non_oceanic_biome_overrides_ocean() {
        let ocean = vec![Biome::WARM_OCEAN; 4 * 4];
        let land = uniform_land(Biome::FOREST.id(), 4, 4);
        let mut out = vec![Biome::NONE; 4 * 4];
        // No bbox expansion needed because we feed land that already
        // covers ocean exactly; warm cells short-circuit on the
        // !is_oceanic check before touching the neighbourhood.
        map_ocean_mix(&ocean, &land, &mut out, 4, 4, 0, 0, 4, 4);
        for cell in &out {
            assert_eq!(*cell, Biome::FOREST);
        }
    }

    #[test]
    fn warm_ocean_with_land_nearby_becomes_lukewarm() {
        let ocean = vec![Biome::WARM_OCEAN; 1];
        let bbox = ocean_land_bbox(&ocean, 1, 1);
        assert_eq!(bbox, (-8, 9, -8, 9));
        let lw = (bbox.1 - bbox.0) as usize;
        let lh = (bbox.3 - bbox.2) as usize;
        let mut land = vec![Biome::OCEAN; lw * lh];
        // (-4, -4) in ocean coords -> (-4 - (-8), -4 - (-8)) = (4, 4)
        // in land coords.
        land[4 + 4 * lw] = Biome::FOREST;
        let mut out = vec![Biome::NONE; 1];
        map_ocean_mix(&ocean, &land, &mut out, 1, 1, bbox.0, bbox.2, lw, lh);
        assert_eq!(out[0], Biome::LUKEWARM_OCEAN);
    }

    #[test]
    fn warm_ocean_with_no_land_nearby_stays_warm() {
        let ocean = vec![Biome::WARM_OCEAN; 1];
        let bbox = ocean_land_bbox(&ocean, 1, 1);
        let lw = (bbox.1 - bbox.0) as usize;
        let lh = (bbox.3 - bbox.2) as usize;
        let land = uniform_land(Biome::OCEAN.id(), lw, lh);
        let mut out = vec![Biome::NONE; 1];
        map_ocean_mix(&ocean, &land, &mut out, 1, 1, bbox.0, bbox.2, lw, lh);
        assert_eq!(out[0], Biome::WARM_OCEAN);
    }

    #[test]
    fn frozen_ocean_with_land_nearby_becomes_cold() {
        let ocean = vec![Biome::FROZEN_OCEAN; 1];
        let bbox = ocean_land_bbox(&ocean, 1, 1);
        let lw = (bbox.1 - bbox.0) as usize;
        let lh = (bbox.3 - bbox.2) as usize;
        let mut land = vec![Biome::OCEAN; lw * lh];
        land[4 + 4 * lw] = Biome::FOREST;
        let mut out = vec![Biome::NONE; 1];
        map_ocean_mix(&ocean, &land, &mut out, 1, 1, bbox.0, bbox.2, lw, lh);
        assert_eq!(out[0], Biome::COLD_OCEAN);
    }

    #[test]
    fn deep_ocean_centre_promotes_lukewarm_to_deep_lukewarm() {
        let ocean = vec![Biome::LUKEWARM_OCEAN; 1];
        let land = uniform_land(Biome::DEEP_OCEAN.id(), 1, 1);
        let mut out = vec![Biome::NONE; 1];
        map_ocean_mix(&ocean, &land, &mut out, 1, 1, 0, 0, 1, 1);
        assert_eq!(out[0], Biome::DEEP_LUKEWARM_OCEAN);
    }

    #[test]
    fn deep_ocean_centre_promotes_ocean_to_deep_ocean() {
        let ocean = vec![Biome::OCEAN; 1];
        let land = uniform_land(Biome::DEEP_OCEAN.id(), 1, 1);
        let mut out = vec![Biome::NONE; 1];
        map_ocean_mix(&ocean, &land, &mut out, 1, 1, 0, 0, 1, 1);
        assert_eq!(out[0], Biome::DEEP_OCEAN);
    }
}
