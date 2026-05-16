//! Debug investigation: compare cubiomes' `gen_biomes` output
//! (dumped via `fixtures-gen debug-monument-cache`) against my Rust
//! `Generator::gen_biomes` for the failing Monument viability
//! `Range`. Localises where the divergence happens.
//!
//! **Conclusion** (manual run 2026-05-16): all 256 cells match
//! between Rust and cubiomes' raw `gen_biomes`. The viability
//! divergence is caused by cubiomes' `mapViableBiome` /
//! `mapViableShore` layer hooks, which short-circuit the layer
//! chain when the `L_BIOME_256` sample area has no oceanic biome.
//! The hook returns `-1` for cells that would otherwise be valid,
//! making cubiomes return "not viable" for some pre-1.18 Monument
//! seeds where the raw biome is in fact deep-ocean.

#![allow(clippy::missing_panics_doc)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::cast_slice;
use cubioxides::biome::Biome;
use cubioxides::generator::{Generator, Range};
use cubioxides::mc_version::{Dimension, MCVersion};

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("debug_monument_cache.bin")
}

#[test]
#[ignore = "diagnostic helper for Monument pre-1.18 divergence"]
fn dump_diff() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let cubiomes_ids: &[i32] = cast_slice(&bytes);

    // Same Range as the failing monument viability case:
    // V1_12 seed=0x72fdd558873e067e, scale=4, x=-978, z=154, sx=16, sz=16, y=8, sy=1.
    let mut g = Generator::new(MCVersion::V1_12, 0);
    g.apply_seed(Dimension::Overworld, 0x72fd_d558_873e_067e);
    let r = Range {
        scale: 4,
        x: -978,
        z: 154,
        sx: 16,
        sz: 16,
        y: 8,
        sy: 1,
    };
    let mut rust_cache = vec![Biome::default(); r.cell_count()];
    g.gen_biomes(&mut rust_cache, r);

    assert_eq!(rust_cache.len(), cubiomes_ids.len());
    let mut diff = 0;
    for i in 0..cubiomes_ids.len() {
        if rust_cache[i].0 != cubiomes_ids[i] {
            if diff < 10 {
                println!(
                    "cell {} (x={}, z={}): rust={}, cubiomes={}",
                    i,
                    r.x + (i as i32 % r.sx as i32),
                    r.z + (i as i32 / r.sx as i32),
                    rust_cache[i].0,
                    cubiomes_ids[i]
                );
            }
            diff += 1;
        }
    }
    println!("Total diff cells: {} / {}", diff, cubiomes_ids.len());
    assert_eq!(diff, 0, "gen_biomes divergence — see stdout for details");
}
