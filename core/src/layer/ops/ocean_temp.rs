//! `mapOceanTemp` — temperature-aware ocean variant assignment.
//!
//! Bit-exact port of cubiomes' `mapOceanTemp`. The layer samples a
//! Perlin noise at `((i + x) / 8.0, (j + z) / 8.0, 0)` and bins the
//! result into five ocean variants: warm > 0.4 > lukewarm > 0.2 >
//! ocean > -0.2 > cold > -0.4 > frozen.

use crate::biome::Biome;
use crate::noise::PerlinNoise;

/// `mapOceanTemp` — output is `w * h` cells, no parent.
///
/// The Perlin noise is owned by the caller; cubiomes derives it from
/// the layer's world seed via `perlinInit` (Java RNG).
pub fn map_ocean_temp(noise: &PerlinNoise, out: &mut [Biome], x: i32, z: i32, w: usize, h: usize) {
    assert!(out.len() >= w * h, "map_ocean_temp: output slice too small");

    for j in 0..h {
        for i in 0..w {
            let tmp = noise.sample(
                f64::from(i as i32 + x) / 8.0,
                f64::from(j as i32 + z) / 8.0,
                0.0,
                0.0,
                0.0,
            );
            let id = if tmp > 0.4 {
                Biome::WARM_OCEAN.id()
            } else if tmp > 0.2 {
                Biome::LUKEWARM_OCEAN.id()
            } else if tmp < -0.4 {
                Biome::FROZEN_OCEAN.id()
            } else if tmp < -0.2 {
                Biome::COLD_OCEAN.id()
            } else {
                Biome::OCEAN.id()
            };
            out[i + j * w] = Biome(id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::JavaRng;

    #[test]
    fn output_is_one_of_five_ocean_variants() {
        let mut rng = JavaRng::new(42);
        let noise = PerlinNoise::from_java(&mut rng);
        let mut out = vec![Biome::NONE; 64 * 64];
        map_ocean_temp(&noise, &mut out, -32, -32, 64, 64);
        for cell in &out {
            assert!(
                matches!(
                    *cell,
                    Biome::WARM_OCEAN
                        | Biome::LUKEWARM_OCEAN
                        | Biome::OCEAN
                        | Biome::COLD_OCEAN
                        | Biome::FROZEN_OCEAN
                ),
                "unexpected biome {cell:?}"
            );
        }
    }

    #[test]
    fn deterministic() {
        let mut rng_a = JavaRng::new(123);
        let noise_a = PerlinNoise::from_java(&mut rng_a);
        let mut rng_b = JavaRng::new(123);
        let noise_b = PerlinNoise::from_java(&mut rng_b);
        let mut a = vec![Biome::NONE; 16];
        let mut b = vec![Biome::NONE; 16];
        map_ocean_temp(&noise_a, &mut a, 0, 0, 4, 4);
        map_ocean_temp(&noise_b, &mut b, 0, 0, 4, 4);
        assert_eq!(a, b);
    }
}
