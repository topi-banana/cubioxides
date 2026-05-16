//! V1_16_1 multi-scale gen_biomes diff. Localises the first
//! upstream layer where Rust diverges from cubiomes.

#![allow(
    clippy::missing_panics_doc,
    clippy::doc_markdown,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::many_single_char_names,
    clippy::uninlined_format_args,
    clippy::items_after_statements
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

fn diff_layer(
    layer_id: cubioxides::layer::LayerId,
    fixture: &str,
    x: i32,
    z: i32,
    sx: usize,
    sz: usize,
) -> usize {
    use cubioxides::layer::{LayerStack, gen_area, set_layer_seed, setup_layer_stack};
    let cubiomes = read_cubiomes(fixture);
    let mut stack = LayerStack::new();
    setup_layer_stack(&mut stack, MCVersion::V1_16_1, false);
    set_layer_seed(&mut stack, layer_id, 0x7edf_7985_db06_7c7d);
    let mut rust = vec![Biome::default(); sx * sz];
    gen_area(&stack, layer_id, &mut rust, x, z, sx, sz);
    let mut d = 0;
    for i in 0..cubiomes.len() {
        if rust[i].0 != cubiomes[i] {
            if d < 16 {
                println!(
                    "  layer cell {i} (x={}, z={}): rust={}, cubiomes={}",
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
#[ignore = "dump parent ring values for mapHills divergence"]
fn dump_parent_rings() {
    let be = read_cubiomes("debug_mch_layer_biome_edge_64_ring.bin");
    let z64h = read_cubiomes("debug_mch_layer_zoom_64_hills_ring.bin");
    println!("--- BiomeEdge64 (biome parent) 6x6 at (-12, -22) ---");
    for row in 0..6 {
        for col in 0..6 {
            print!("{:>4} ", be[(row * 6 + col) as usize]);
        }
        println!();
    }
    println!("--- Zoom64Hills (river parent) 6x6 at (-12, -22) ---");
    for row in 0..6 {
        for col in 0..6 {
            print!("{:>4} ", z64h[(row * 6 + col) as usize]);
        }
        println!();
    }
    // Get the actual L_HILLS_64 layer's start_seed and start_salt.
    use cubioxides::layer::{LayerId, LayerStack, set_layer_seed, setup_layer_stack};
    let mut stack = LayerStack::new();
    setup_layer_stack(&mut stack, MCVersion::V1_16_1, false);
    set_layer_seed(&mut stack, LayerId::Hills64, 0x7edf_7985_db06_7c7d);
    let hills_node = stack.node(LayerId::Hills64);
    println!(
        "Hills64 start_salt={:x} start_seed={:x} layer_salt={:x}",
        hills_node.start_salt, hills_node.start_seed, hills_node.layer_salt
    );

    // Focus on the diverging cells only.
    println!("--- DIVERGING cells: a11 + 4 neighbors + chunk seed ---");
    for (i, j) in [(2_i32, 1_i32), (3_i32, 3_i32)] {
        let a11 = be[((i + 1) + (j + 1) * 6) as usize];
        let a10 = be[((i + 1) + j * 6) as usize];
        let a21 = be[((i + 2) + (j + 1) * 6) as usize];
        let a01 = be[(i + (j + 1) * 6) as usize];
        let a12 = be[((i + 1) + (j + 2) * 6) as usize];
        let b11 = z64h[((i + 1) + (j + 1) * 6) as usize];
        let bn = if b11 >= 0 {
            (b11 - 2).rem_euclid(29)
        } else {
            -1
        };
        let cs = cubioxides::rng::get_chunk_seed(hills_node.start_seed, i - 11, j - 21);
        let first0_3 = cubioxides::rng::mc_first_is_zero(cs, 3);
        println!(
            "  cell (x={}, z={}) a11={} a10={} a21={} a01={} a12={} b11={} bn={} cs={:x} first0_3={}",
            i - 11,
            j - 21,
            a11,
            a10,
            a21,
            a01,
            a12,
            b11,
            bn,
            cs,
            first0_3
        );
    }
}

#[test]
#[ignore = "diagnostic helper for V1_16_1 BiomeEdge64 / Hills64 divergence"]
fn layer_diff() {
    use cubioxides::layer::LayerId;
    println!("--- BiomeEdge64 (L_BIOME_EDGE_64 = 25) 4x4 ---");
    let d_be = diff_layer(
        LayerId::BiomeEdge64,
        "debug_mch_layer_biome_edge_64.bin",
        -11,
        -21,
        4,
        4,
    );
    println!("  total diff BiomeEdge64 inner: {}/16", d_be);
    println!("--- BiomeEdge64 6x6 ring ---");
    let d_be_ring = diff_layer(
        LayerId::BiomeEdge64,
        "debug_mch_layer_biome_edge_64_ring.bin",
        -12,
        -22,
        6,
        6,
    );
    println!("  total diff BiomeEdge64 ring: {}/36", d_be_ring);
    println!("--- Zoom64Hills 6x6 ring ---");
    let d_z = diff_layer(
        LayerId::Zoom64Hills,
        "debug_mch_layer_zoom_64_hills_ring.bin",
        -12,
        -22,
        6,
        6,
    );
    println!("  total diff Zoom64Hills ring: {}/36", d_z);
    println!("--- Hills64 (L_HILLS_64 = 29) ---");
    let d_h = diff_layer(
        LayerId::Hills64,
        "debug_mch_layer_hills_64.bin",
        -11,
        -21,
        4,
        4,
    );
    println!("  total diff Hills64: {}/16", d_h);
}
