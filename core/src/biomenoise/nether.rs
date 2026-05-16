//! 1.16+ Nether biome noise (`NetherNoise`).
//!
//! Bit-exact port of cubiomes' `setNetherSeed`, `getNetherBiome`,
//! `mapNether3D` (with the `fillRad3D` confidence-radius
//! optimisation), and `mapNether2D`. Two `DoublePerlinNoise` stacks
//! (temperature, humidity) drive a nearest-anchor lookup over five
//! Nether biome anchor points.
//!
//! Note: cubiomes computes the distance calculation in `float`, not
//! `double`. The Rust port mirrors this by casting both noise samples
//! to `f32` immediately and doing all anchor-distance arithmetic in
//! `f32`, so the rounding behaviour matches exactly.

#![allow(clippy::many_single_char_names)]

use crate::biome::Biome;
use crate::noise::DoublePerlinNoise;
use crate::rng::JavaRng;

/// 1.16+ Nether biome noise. Built by [`NetherNoise::set_seed`].
#[derive(Debug, Clone)]
pub struct NetherNoise {
    /// Temperature double-Perlin stack (-7..-6, 2 octaves).
    pub temperature: DoublePerlinNoise,
    /// Humidity double-Perlin stack (-7..-6, 2 octaves).
    pub humidity: DoublePerlinNoise,
}

/// Five Nether biome anchor points: `(temperature, humidity,
/// weight_offset, biome_id)`. Mirrors the `npoints` array in
/// `cubiomes/biomenoise.c::getNetherBiome`.
///
/// Cubiomes stores the array as `const float[5][4]`; the
/// `0.375*0.375` / `0.175*0.175` weights are computed in `double`
/// (the default C literal type) and then narrowed to `float` at
/// store time. Mirror that exactly here by computing in `f64` and
/// casting to `f32` — a plain `0.175_f32 * 0.175_f32` would round
/// differently by 1 ULP.
const NPOINTS: [(f32, f32, f32, i32); 5] = [
    (0.0, 0.0, 0.0, 8),                               // nether_wastes
    (0.0, -0.5, 0.0, 170),                            // soul_sand_valley
    (0.4, 0.0, 0.0, 171),                             // crimson_forest
    (0.0, 0.5, (0.375_f64 * 0.375_f64) as f32, 172),  // warped_forest
    (-0.5, 0.0, (0.175_f64 * 0.175_f64) as f32, 173), // basalt_deltas
];

impl NetherNoise {
    /// `setNetherSeed(nn, seed)` — re-init temperature + humidity
    /// from a Java RNG keyed by `seed` and `seed + 1` respectively.
    ///
    /// # Example
    ///
    /// ```
    /// use cubioxides::biomenoise::NetherNoise;
    ///
    /// // The 1.16+ Nether biomes are Voronoi-classified by two
    /// // f32 axes; `set_seed` initialises the climate noise that
    /// // feeds those axes.
    /// let nn = NetherNoise::set_seed(0xdead_beef);
    /// let (_biome, _ndel) = nn.get_nether_biome(0, 64, 0);
    /// ```
    #[must_use]
    pub fn set_seed(seed: u64) -> Self {
        let mut rng = JavaRng::new(seed);
        let temperature = DoublePerlinNoise::from_java(&mut rng, -7, 2);
        let mut rng = JavaRng::new(seed.wrapping_add(1));
        let humidity = DoublePerlinNoise::from_java(&mut rng, -7, 2);
        Self {
            temperature,
            humidity,
        }
    }

    /// Cubiomes' `getNetherBiome(nn, x, y, z, ndel)`. Returns
    /// `(biome, ndel)` where `ndel = sqrt(dmin2) - sqrt(dmin)` —
    /// the gap between the closest and second-closest anchor (used
    /// by [`Self::map_nether_3d`] to fill a confidence radius).
    #[must_use]
    pub fn get_nether_biome(&self, x: i32, _y: i32, z: i32) -> (Biome, f32) {
        // cubiomes forces y = 0 inside getNetherBiome — Nether biomes
        // are 2D over the (x, z) plane.
        let temp = self.temperature.sample(f64::from(x), 0.0, f64::from(z)) as f32;
        let humidity = self.humidity.sample(f64::from(x), 0.0, f64::from(z)) as f32;

        let mut id_idx = 0;
        let mut dmin = f32::MAX;
        let mut dmin2 = f32::MAX;
        for (i, &(tx, ty, weight, _)) in NPOINTS.iter().enumerate() {
            let dx = tx - temp;
            let dy = ty - humidity;
            let dsq = dx * dx + dy * dy + weight;
            if dsq < dmin {
                dmin2 = dmin;
                dmin = dsq;
                id_idx = i;
            } else if dsq < dmin2 {
                dmin2 = dsq;
            }
        }
        let ndel = dmin2.sqrt() - dmin.sqrt();
        (Biome(NPOINTS[id_idx].3), ndel)
    }

    /// Cubiomes' `mapNether3D(nn, out, range, confidence)`. Fills
    /// `out` with biome ids over a 3D `(sx * sy * sz)` cuboid at
    /// scale `range.scale`. The `confidence` factor controls the
    /// radius around each sample within which neighbouring voxels
    /// inherit the same biome (1.0 = cubiomes' default).
    ///
    /// `range.scale` must be at least 4.
    pub fn map_nether_3d(
        &self,
        out: &mut [i32],
        x: i32,
        y: i32,
        z: i32,
        sx: usize,
        sy: usize,
        sz: usize,
        scale: i32,
        confidence: f32,
    ) {
        assert!(scale >= 4, "map_nether_3d requires scale >= 4");
        let sy = sy.max(1);
        let total = sx * sy * sz;
        out[..total].fill(0);

        let inner_scale = scale / 4;
        let invgrad = 1.0 / (confidence * 0.05 * 2.0) / inner_scale as f32;

        for k in 0..sy as i32 {
            for j in 0..sz as i32 {
                for i in 0..sx as i32 {
                    let idx = k as usize * sx * sz + j as usize * sx + i as usize;
                    if out[idx] != 0 {
                        continue;
                    }

                    let xi = (x + i) * inner_scale;
                    let yk = y + k;
                    let zj = (z + j) * inner_scale;
                    let (biome, noisedelta) = self.get_nether_biome(xi, yk, zj);
                    out[idx] = biome.id();
                    let rad = noisedelta * invgrad - 1.0;
                    if rad > 0.0 {
                        fill_rad_3d(out, i, k, j, sx, sy, sz, biome.id(), rad);
                    }
                }
            }
        }
    }

    /// Cubiomes' `mapNether2D(nn, out, x, z, w, h)` — fixed scale =
    /// 4, sy = 1, y = 0.
    pub fn map_nether_2d(&self, out: &mut [i32], x: i32, z: i32, w: usize, h: usize) {
        self.map_nether_3d(out, x, 0, z, w, 1, h, 4, 1.0);
    }
}

/// Flood-fill a sphere of radius `rad` (in voxels) around
/// `(cx, cy, cz)` with `id`, clamped to the `(sx, sy, sz)` cuboid.
/// Mirrors cubiomes' static `fillRad3D`. `rad` ≤ 0 is a no-op.
fn fill_rad_3d(
    out: &mut [i32],
    cx: i32,
    cy: i32,
    cz: i32,
    sx: usize,
    sy: usize,
    sz: usize,
    id: i32,
    rad: f32,
) {
    let r = rad as i32;
    if r <= 0 {
        return;
    }
    // floor(rad * rad) — cubiomes uses (int)floor(rad * rad)
    let rsq = (rad * rad).floor() as i32;

    for k in -r..=r {
        let ak = cy + k;
        if ak < 0 || ak as usize >= sy {
            continue;
        }
        let ksq = k * k;
        let layer_base = ak as usize * sx * sz;

        for j in -r..=r {
            let aj = cz + j;
            if aj < 0 || aj as usize >= sz {
                continue;
            }
            let jksq = j * j + ksq;
            for i in -r..=r {
                let ai = cx + i;
                if ai < 0 || ai as usize >= sx {
                    continue;
                }
                if i * i + jksq > rsq {
                    continue;
                }
                out[layer_base + aj as usize * sx + ai as usize] = id;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_seed_deterministic() {
        let a = NetherNoise::set_seed(0xdead_beef);
        let b = NetherNoise::set_seed(0xdead_beef);
        let s1 = a.get_nether_biome(0, 0, 0);
        let s2 = b.get_nether_biome(0, 0, 0);
        assert_eq!(s1.0, s2.0);
        assert_eq!(s1.1.to_bits(), s2.1.to_bits());
    }

    #[test]
    fn anchor_biomes_are_in_valid_range() {
        let nn = NetherNoise::set_seed(1);
        for x in (-128..128).step_by(8) {
            for z in (-128..128).step_by(8) {
                let (b, _) = nn.get_nether_biome(x, 0, z);
                assert!(
                    matches!(
                        b,
                        Biome::NETHER_WASTES
                            | Biome::SOUL_SAND_VALLEY
                            | Biome::CRIMSON_FOREST
                            | Biome::WARPED_FOREST
                            | Biome::BASALT_DELTAS
                    ),
                    "unexpected biome {b:?} at ({x}, {z})"
                );
            }
        }
    }

    #[test]
    fn map_nether_2d_fills_grid() {
        let nn = NetherNoise::set_seed(42);
        let w = 16;
        let h = 16;
        let mut out = vec![0i32; w * h];
        nn.map_nether_2d(&mut out, 0, 0, w, h);
        // Every cell should have a nether biome id.
        for v in &out {
            assert!([8, 170, 171, 172, 173].contains(v));
        }
    }
}
