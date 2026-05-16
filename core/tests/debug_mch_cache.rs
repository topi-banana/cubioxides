//! Debug investigation: V1_16_1 `gen_biomes` parity for the
//! `map_approx_height_legacy` failure case (stage 14b excluded
//! V1_16_1; this localises whether the bug is in `gen_biomes`).
//!
//! **Conclusion** (manual run 2026-05-16): 107 / 169 cells diverge
//! between Rust and cubiomes at V1_16_1, even after the
//! `are_similar_ids` `is_before(V1_16_1)` fix. The remaining
//! divergence is upstream of `mapBiomeEdge` / `mapHills` (the
//! V1_16_1 layer-stack setup or an earlier op). Follow-up
//! investigation needed to localise to a specific layer.

#![allow(clippy::missing_panics_doc, clippy::doc_markdown)]

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
        .join("debug_mch_cache.bin")
}

#[test]
#[ignore = "diagnostic helper for V1_16_1 gen_biomes divergence"]
fn dump_diff() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let cubiomes_ids: &[i32] = cast_slice(&bytes);

    // Same Range as the failing case:
    // V1_16_1 seed=0x7edf7985db067c7d, scale=4, x=-158, z=-327,
    // sx=sz=13, y=0, sy=1.
    let mut g = Generator::new(MCVersion::V1_16_1, 0);
    g.apply_seed(Dimension::Overworld, 0x7edf_7985_db06_7c7d);
    let r = Range {
        scale: 4,
        x: -158,
        z: -327,
        sx: 13,
        sz: 13,
        y: 0,
        sy: 1,
    };
    let mut rust_cache = vec![Biome::default(); r.cell_count()];
    g.gen_biomes(&mut rust_cache, r);

    assert_eq!(rust_cache.len(), cubiomes_ids.len());
    let mut diff = 0;
    for i in 0..cubiomes_ids.len() {
        if rust_cache[i].0 != cubiomes_ids[i] {
            if diff < 20 {
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
    assert_eq!(diff, 0, "V1_16_1 gen_biomes divergence");
}
