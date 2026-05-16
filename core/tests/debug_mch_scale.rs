//! V1_16_1 multi-scale gen_biomes diff. Localises the first
//! upstream layer where Rust diverges from cubiomes.

#![allow(
    clippy::missing_panics_doc,
    clippy::doc_markdown,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::uninlined_format_args
)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::cast_slice;
use cubioxides::biome::Biome;
use cubioxides::generator::{Generator, Range};
use cubioxides::mc_version::{Dimension, MCVersion};

fn read_cubiomes(name: &str) -> Vec<i32> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join(name);
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let ids: &[i32] = cast_slice(&bytes);
    ids.to_vec()
}

fn diff(scale: i32, x: i32, z: i32, sx: u32, sz: u32, fixture: &str) -> usize {
    let cubiomes = read_cubiomes(fixture);
    let mut g = Generator::new(MCVersion::V1_16_1, 0);
    g.apply_seed(Dimension::Overworld, 0x7edf_7985_db06_7c7d);
    let r = Range {
        scale,
        x,
        z,
        sx,
        sz,
        y: 0,
        sy: 1,
    };
    let mut rust = vec![Biome::default(); r.cell_count()];
    g.gen_biomes(&mut rust, r);
    let mut d = 0;
    for i in 0..cubiomes.len() {
        if rust[i].0 != cubiomes[i] {
            if d < 8 {
                println!(
                    "  scale={scale} cell {i} (x={}, z={}): rust={}, cubiomes={}",
                    x + (i as i32 % sx as i32),
                    z + (i as i32 / sx as i32),
                    rust[i].0,
                    cubiomes[i]
                );
            }
            d += 1;
        }
    }
    d
}

#[test]
#[ignore = "diagnostic helper for V1_16_1 layer-stack divergence"]
fn scale_diff() {
    println!("--- scale 256 ---");
    let d256 = diff(256, -3, -6, 3, 3, "debug_mch_scale256.bin");
    println!("  total diff at scale 256: {}/9", d256);
    println!("--- scale 64 ---");
    let d64 = diff(64, -11, -21, 4, 4, "debug_mch_scale64.bin");
    println!("  total diff at scale 64: {}/16", d64);
    println!("--- scale 16 ---");
    let d16 = diff(16, -41, -83, 8, 8, "debug_mch_scale16.bin");
    println!("  total diff at scale 16: {}/64", d16);
}
