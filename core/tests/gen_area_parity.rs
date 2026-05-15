//! Parity test: `cubioxides::layer::gen_area` vs cubiomes' `genArea`.
//! Reads the binary fixture produced by `fixtures-gen layers`
//! (kind = 38) and compares the XOR-folded digest of the output grid
//! at a variety of `(mc, layer_id, world_seed, x, z, w, h)` tuples
//! that span every dispatch arm of the Rust `LayerOp` switch (MC 1.18
//! DAG; other MC versions land in a follow-up commit).

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
const KIND: u16 = 38;

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
struct GenAreaRecord {
    mc: u32,
    large_biomes: u32,
    world_seed: u64,
    layer_id: u32,
    x: i32,
    z: i32,
    w: u32,
    h: u32,
    digest: u32,
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("fixtures")
        .join("layers")
        .join("gen_area.bin")
}

fn mc_from_ord(ord: u32) -> MCVersion {
    match ord {
        22 => MCVersion::V1_18,
        other => panic!("unsupported MC ordinal: {other}"),
    }
}

fn layer_id_from_ord(ord: u32) -> cubioxides::layer::LayerId {
    use cubioxides::layer::LayerId::*;
    match ord {
        0 => Continent4096,
        3 => Zoom2048,
        4 => Land2048,
        10 => Snow1024,
        12 => Cool1024,
        14 => Special1024,
        19 => Mushroom256,
        20 => DeepOcean256,
        21 => Biome256,
        22 => Bamboo256,
        25 => BiomeEdge64,
        26 => Noise256,
        29 => Hills64,
        30 => Sunflower64,
        34 => Shore16,
        38 => Smooth4,
        45 => River4,
        47 => RiverMix4,
        55 => OceanMix4,
        56 => Voronoi1,
        other => panic!("unsupported layer id: {other}"),
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
fn gen_area_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (header_bytes, body_bytes) = bytes.split_at(std::mem::size_of::<Header>());
    let header: &Header = bytemuck::from_bytes(header_bytes);
    assert_eq!(header.magic, MAGIC);
    assert_eq!(header.format_version, FORMAT_VERSION);
    assert_eq!(header.kind, KIND);
    let records: &[GenAreaRecord] = bytemuck::cast_slice(body_bytes);
    assert_eq!(records.len() as u64, header.record_count);
    assert!(!records.is_empty());

    let mut stack = Box::new(LayerStack::new());
    for (i, rec) in records.iter().enumerate() {
        let mc = mc_from_ord(rec.mc);
        let large_biomes = rec.large_biomes != 0;
        let layer = layer_id_from_ord(rec.layer_id);

        setup_layer_stack(&mut stack, mc, large_biomes);
        let entry = stack.entry_1.expect("entry_1");
        set_layer_seed(&mut stack, entry, rec.world_seed);

        let w = rec.w as usize;
        let h = rec.h as usize;
        let mut out = vec![Biome::NONE; w * h];
        gen_area(&stack, layer, &mut out, rec.x, rec.z, w, h);

        let mut digest: u32 = 0;
        for cell in &out {
            digest ^= hash32(cell.id() as u32);
        }
        assert_eq!(
            digest, rec.digest,
            "gen_area digest mismatch at record {i} \
             (mc={}, layer_id={}, world={:#x}, x={}, z={}, w={}, h={})",
            rec.mc, rec.layer_id, rec.world_seed, rec.x, rec.z, rec.w, rec.h
        );
    }
}
