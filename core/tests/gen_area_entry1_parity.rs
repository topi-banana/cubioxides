//! Parity test: cubioxides' `gen_area` at `entry_1` (Voronoi) vs
//! cubiomes' `genArea` across the MC version matrix B1.8 / 1.0 / 1.1
//! / 1.6 / 1.7 / 1.12 / 1.13 / 1.14 / 1.18 / 1.20. Each record runs
//! the full DAG for the requested version, so any divergence in
//! `setup_layer_stack`, `set_layer_seed`, or any `LayerOp` dispatch
//! arm shows up here.

#![allow(clippy::missing_panics_doc)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::biome::Biome;
use cubioxides::layer::{LayerStack, gen_area, set_layer_seed, setup_layer_stack};
use cubioxides::mc_version::MCVersion;

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 39;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Header {
    magic: [u8; 4],
    format_version: u16,
    kind: u16,
    record_count: u64,
    reserved: [u64; 2],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct GenAreaEntry1Record {
    mc: u32,
    large_biomes: u32,
    world_seed: u64,
    x: i32,
    z: i32,
    w: u32,
    h: u32,
    digest: u32,
    pad: u32,
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("fixtures")
        .join("layers")
        .join("gen_area_entry1.bin")
}

fn mc_from_ord(ord: u32) -> MCVersion {
    match ord {
        2 => MCVersion::B1_8,
        3 => MCVersion::V1_0,
        4 => MCVersion::V1_1,
        9 => MCVersion::V1_6,
        10 => MCVersion::V1_7,
        15 => MCVersion::V1_12,
        16 => MCVersion::V1_13,
        17 => MCVersion::V1_14,
        22 => MCVersion::V1_18,
        25 => MCVersion::V1_20,
        other => panic!("unsupported MC ordinal: {other}"),
    }
}

fn hash32(mut x: u32) -> u32 {
    x ^= x >> 15;
    x = x.wrapping_mul(0xd168_aaad);
    x ^= x >> 15;
    x = x.wrapping_mul(0xaf72_3597);
    x ^= x >> 15;
    x
}

#[test]
fn gen_area_at_entry1_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (header_bytes, body_bytes) = bytes.split_at(std::mem::size_of::<Header>());
    let header: &Header = bytemuck::from_bytes(header_bytes);
    assert_eq!(header.magic, MAGIC);
    assert_eq!(header.format_version, FORMAT_VERSION);
    assert_eq!(header.kind, KIND);
    let records: &[GenAreaEntry1Record] = bytemuck::cast_slice(body_bytes);
    assert_eq!(records.len() as u64, header.record_count);
    assert!(!records.is_empty());

    let mut stack = Box::new(LayerStack::new());
    for (i, rec) in records.iter().enumerate() {
        let mc = mc_from_ord(rec.mc);
        let large_biomes = rec.large_biomes != 0;

        setup_layer_stack(&mut stack, mc, large_biomes);
        let entry = stack.entry_1.expect("entry_1");
        set_layer_seed(&mut stack, entry, rec.world_seed);

        let w = rec.w as usize;
        let h = rec.h as usize;
        let mut out = vec![Biome::NONE; w * h];
        gen_area(&stack, entry, &mut out, rec.x, rec.z, w, h);

        let mut digest: u32 = 0;
        for cell in &out {
            digest ^= hash32(cell.id() as u32);
        }
        assert_eq!(
            digest, rec.digest,
            "gen_area@entry_1 digest mismatch at record {i} \
             (mc={}, large_biomes={}, world={:#x}, x={}, z={}, w={}, h={})",
            rec.mc, rec.large_biomes, rec.world_seed, rec.x, rec.z, rec.w, rec.h
        );
    }
}
