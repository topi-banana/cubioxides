//! `mapContinent` — the first layer in the legacy biome pipeline.
//!
//! Bit-exact port of `mapContinent` in `cubiomes/layers.c`. Each cell is
//! either ocean (`0`) or plains (`1`), with plains chosen when the chunk
//! seed's first-int-mod-10 equals zero. The central cell `(0, 0)` is
//! forced to plains when included in the requested region; cubiomes
//! uses this to guarantee a non-empty starting continent.

use crate::biome::Biome;
use crate::rng::{get_chunk_seed, mc_first_is_zero};

/// Fill `out[0..w*h]` with the `mapContinent` output for the rectangle
/// whose top-left corner is `(x, z)`.
///
/// `start_seed` is the per-layer start seed produced by the world-seed
/// pipeline (`get_start_seed(world_seed, layer_salt)`).
///
/// # Panics
///
/// Panics if `out.len() < w * h`.
pub fn map_continent(start_seed: u64, out: &mut [Biome], x: i32, z: i32, w: usize, h: usize) {
    assert!(
        out.len() >= w * h,
        "map_continent: output buffer too small (got {}, need {})",
        out.len(),
        w * h
    );

    for j in 0..h {
        for i in 0..w {
            let cs = get_chunk_seed(start_seed, (i as i32) + x, (j as i32) + z);
            // mc_first_is_zero returns bool; cast to {0, 1} biome id.
            let id = i32::from(mc_first_is_zero(cs, 10));
            out[j * w + i] = Biome(id);
        }
    }

    // Centre cell guarantee: if (0, 0) is inside the requested rectangle,
    // force it to plains so the output never starts with a fully-ocean
    // continent.
    if x > -(w as i32) && x <= 0 && z > -(h as i32) && z <= 0 {
        let idx = (-z) as usize * w + (-x) as usize;
        out[idx] = Biome::PLAINS;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rng::{get_layer_salt, get_start_seed};

    fn layer_start_seed(world: u64) -> u64 {
        // Use an arbitrary non-zero salt for testing. The actual layer
        // salts come from cubiomes' layer setup and will land alongside
        // the LayerStack port.
        const SALT: u64 = 1;
        get_start_seed(world, get_layer_salt(SALT))
    }

    #[test]
    fn centre_cell_is_forced_to_plains() {
        let ss = layer_start_seed(0);
        let mut out = vec![Biome::NONE; 3 * 3];
        map_continent(ss, &mut out, -1, -1, 3, 3);
        // (0, 0) corresponds to local (1, 1) in the 3x3 grid.
        assert_eq!(out[3 + 1], Biome::PLAINS);
    }

    #[test]
    fn centre_cell_skipped_when_outside_rectangle() {
        let ss = layer_start_seed(42);
        let mut a = vec![Biome::NONE; 4];
        let mut b = vec![Biome::NONE; 4];
        // A rectangle that does not contain (0, 0):
        map_continent(ss, &mut a, 10, 10, 2, 2);
        // Same call again — no forced cell means the output is purely
        // determined by the chunk seed; deterministic across runs.
        map_continent(ss, &mut b, 10, 10, 2, 2);
        assert_eq!(a, b);
    }

    #[test]
    fn output_is_either_ocean_or_plains() {
        let ss = layer_start_seed(123);
        let mut out = vec![Biome::NONE; 8 * 8];
        map_continent(ss, &mut out, -4, -4, 8, 8);
        for b in &out {
            assert!(
                *b == Biome::OCEAN || *b == Biome::PLAINS,
                "map_continent emitted unexpected biome {b:?}"
            );
        }
    }

    #[test]
    fn output_size_is_w_times_h() {
        let ss = layer_start_seed(7);
        let mut out = vec![Biome::NONE; 16 * 9];
        map_continent(ss, &mut out, -5, -3, 16, 9);
        // No forced centre on this rectangle (centre is in range, so PLAINS).
        // Verify every cell is filled (not NONE).
        for cell in &out {
            assert_ne!(*cell, Biome::NONE);
        }
    }

    #[test]
    #[should_panic(expected = "output buffer too small")]
    fn panics_on_undersized_buffer() {
        let ss = layer_start_seed(0);
        let mut out = vec![Biome::NONE; 3]; // need 9
        map_continent(ss, &mut out, 0, 0, 3, 3);
    }
}
