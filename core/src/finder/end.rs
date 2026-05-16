//! End-dimension decorator features.
//!
//! Bit-exact port of cubiomes' `getEndIslands` from `finders.c`. Each
//! End chunk has a chance (~1/14) to generate one or two floating
//! Y-islands, with a per-island center `(x, y, z)` and a footprint
//! radius `r` (4..=6).
//!
//! Three MC-version branches:
//!
//! - MC ≤ 1.16: integer rarity (14), Java RNG, `nextInt(rng, 14) != 0`
//!   skips. Second island via `nextInt(rng, 4) != 0` (3-in-4 chance).
//! - MC ≤ 1.17: float rarity (`1/14`), Java RNG, `nextFloat(&rng) >= rarity`
//!   skips. Second island via `nextInt(rng, 4) == 0` (1-in-4 chance).
//! - MC ≥ 1.18: Xoroshiro, `xNextFloat(&xr) >= rarity` skips. Second
//!   island via `xNextIntJ(&xr, 4) == 3` (1-in-4 chance).
//!
//! Subsequent commits add `mapEndIslandHeight` and the `EndNoise`
//! integration that turns these islands into actual terrain heights.

use crate::biome::Biome;
use crate::biomenoise::end::EndNoise;
use crate::finder::{StructureType, get_structure_config};
use crate::math::floordiv;
use crate::mc_version::MCVersion;
use crate::rng::{JavaRng, Xoroshiro};

/// One floating island in the End dimension. Mirrors cubiomes'
/// `STRUCT(EndIsland)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct EndIsland {
    /// Centre block-X.
    pub x: i32,
    /// Centre block-Y (always in 55..=70).
    pub y: i32,
    /// Centre block-Z.
    pub z: i32,
    /// Footprint radius (4..=6).
    pub r: i32,
}

/// `getEndIslands(islands, mc, seed, chunkX, chunkZ)` — fill `islands`
/// (capacity 2) with the End-island centres for the given chunk and
/// return the count (0, 1, or 2). Returns 0 for MC < 1.13 since
/// `getStructureConfig(End_Island, mc, …)` reports unsupported.
#[must_use]
pub fn get_end_islands(
    islands: &mut [EndIsland; 2],
    mc: MCVersion,
    seed: u64,
    chunk_x: i32,
    chunk_z: i32,
) -> usize {
    let Some(sconf) = get_structure_config(StructureType::EndIsland, mc) else {
        return 0;
    };
    let x = chunk_x * 16;
    let z = chunk_z * 16;
    let rng = super::population_seed::get_population_seed(mc, seed, x, z);
    let salt_u = sconf.salt as i64 as u64;

    if mc.is_before(MCVersion::V1_17) {
        let mut r = JavaRng::new(rng.wrapping_add(salt_u));
        if r.next_int(sconf.rarity as i32) != 0 {
            return 0;
        }
        islands[0].x = r.next_int(16) + x;
        islands[0].y = r.next_int(16) + 55;
        islands[0].z = r.next_int(16) + z;
        if r.next_int(4) != 0 {
            islands[0].r = r.next_int(3) + 4;
            return 1;
        }
        islands[1].x = r.next_int(16) + x;
        islands[1].y = r.next_int(16) + 55;
        islands[1].z = r.next_int(16) + z;
        islands[0].r = r.next_int(3) + 4;
        // cubiomes burns one `nextInt(rng, 2)+0.5` step per `r` units;
        // the loop shape is `for (rf = r; rf > 0.5; rf -= ...);`
        let mut rf = islands[0].r as f32;
        while rf > 0.5 {
            rf -= r.next_int(2) as f32 + 0.5;
        }
        islands[1].r = r.next_int(3) + 4;
        2
    } else if mc.is_before(MCVersion::V1_18) {
        let mut r = JavaRng::new(rng.wrapping_add(salt_u));
        if r.next_float() >= sconf.rarity {
            return 0;
        }
        let second = r.next_int(4) == 0;
        islands[0].x = r.next_int(16) + x;
        islands[0].z = r.next_int(16) + z;
        islands[0].y = r.next_int(16) + 55;
        islands[0].r = r.next_int(3) + 4;
        let mut rf = islands[0].r as f32;
        while rf > 0.5 {
            rf -= r.next_int(2) as f32 + 0.5;
        }
        if !second {
            return 1;
        }
        islands[1].x = r.next_int(16) + x;
        islands[1].z = r.next_int(16) + z;
        islands[1].y = r.next_int(16) + 55;
        islands[1].r = r.next_int(3) + 4;
        2
    } else {
        let mut xr = Xoroshiro::new(rng.wrapping_add(salt_u));
        if xr.next_float() >= sconf.rarity {
            return 0;
        }
        let second = xr.next_int_j(4) == 3;
        islands[0].x = xr.next_int_j(16) + x;
        islands[0].z = xr.next_int_j(16) + z;
        islands[0].y = xr.next_int_j(16) + 55;
        islands[0].r = xr.next_int_j(3) + 4;
        if !second {
            return 1;
        }
        let mut rf = islands[0].r as f32;
        while rf > 0.5 {
            rf -= xr.next_int_j(2) as f32 + 0.5;
        }
        islands[1].x = xr.next_int_j(16) + x;
        islands[1].z = xr.next_int_j(16) + z;
        islands[1].y = xr.next_int_j(16) + 55;
        islands[1].r = xr.next_int_j(3) + 4;
        2
    }
}

/// Raise `y[(j-z)*w + (i-x)]` to `island.y` for every grid cell
/// covered by the island's disk. Mirrors cubiomes' static
/// `applyEndIslandHeight`. The `scale` parameter is 1, 4, or 16 in
/// cubiomes; floor-division semantics over `scale` give the chunk-
/// or block-corner alignment that cubiomes uses.
fn apply_end_island_height(
    y: &mut [f32],
    island: &EndIsland,
    x: i32,
    z: i32,
    w: i32,
    h: i32,
    scale: i32,
) {
    let r = island.r;
    let r2 = (r + 1) * (r + 1);
    let x0 = floordiv(island.x - r, scale);
    let z0 = floordiv(island.z - r, scale);
    let x1 = floordiv(island.x + r, scale);
    let z1 = floordiv(island.z + r, scale);
    // cubiomes uses `ds = 0` here — kept as a literal to mirror the
    // identical add-zero ops in the C source.
    let ds = 0;
    for j in z0..=z1 {
        if j < z || j >= z + h {
            continue;
        }
        let dz = j * scale - island.z + ds;
        for i in x0..=x1 {
            if i < x || i >= x + w {
                continue;
            }
            let dx = i * scale - island.x + ds;
            if dx * dx + dz * dz > r2 {
                continue;
            }
            let idx = ((j - z) * w + (i - x)) as usize;
            if y[idx] < island.y as f32 {
                y[idx] = island.y as f32;
            }
        }
    }
}

/// `mapEndIslandHeight(y, en, seed, x, z, w, h, scale)` — overlay
/// floating-island heights onto `y` (which the caller initialised
/// from the End noise). Mirrors cubiomes' `mapEndIslandHeight`.
///
/// `(x, z, w, h)` is the region of `y` in the scaled coordinate
/// system (block coordinates / `scale`). The function expands the
/// region to chunk coordinates with a `+rmax` margin and applies
/// every `small_end_islands`-tagged chunk's islands.
///
/// Returns 0 to match cubiomes' return value (always 0 in the C
/// version; the API reserves the slot for future error codes).
pub fn map_end_island_height(
    y: &mut [f32],
    en: &EndNoise,
    seed: u64,
    x: i32,
    z: i32,
    w: i32,
    h: i32,
    scale: i32,
) -> i32 {
    assert!(
        scale == 1 || scale == 4 || scale == 16,
        "scale must be 1, 4, or 16"
    );
    assert!(
        y.len() >= (w * h) as usize,
        "map_end_island_height: y buffer too small ({}x{}={} vs len {})",
        w,
        h,
        w * h,
        y.len()
    );

    let rmax = (6 + scale - 1) / scale;
    let step = 16 / scale;
    let cx = floordiv(x - rmax, step);
    let cz = floordiv(z - rmax, step);
    let cw = floordiv(x + w + rmax, step) - cx + 1;
    let ch = floordiv(z + h + rmax, step) - cz + 1;

    let mut ids = vec![0_i32; (cw * ch) as usize];
    en.map_end_biome(&mut ids, cx, cz, cw as usize, ch as usize);
    let target = Biome::SMALL_END_ISLANDS.id();

    for cj in 0..ch {
        for ci in 0..cw {
            if ids[(cj * cw + ci) as usize] != target {
                continue;
            }
            let mut islands = [EndIsland::default(); 2];
            let n = get_end_islands(&mut islands, en.mc, seed, cx + ci, cz + cj);
            for island in islands.iter().take(n) {
                apply_end_island_height(y, island, x, z, w, h, scale);
            }
        }
    }
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pre_1_13_returns_zero_islands() {
        let mut islands = [EndIsland::default(); 2];
        assert_eq!(get_end_islands(&mut islands, MCVersion::V1_12, 1, 0, 0), 0);
    }

    #[test]
    fn deterministic_within_chunk() {
        let mut a = [EndIsland::default(); 2];
        let mut b = [EndIsland::default(); 2];
        let na = get_end_islands(&mut a, MCVersion::V1_18, 0xdead_beef, 5, 7);
        let nb = get_end_islands(&mut b, MCVersion::V1_18, 0xdead_beef, 5, 7);
        assert_eq!(na, nb);
        assert_eq!(a, b);
    }
}
