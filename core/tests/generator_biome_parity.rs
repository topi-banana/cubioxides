//! Parity test: cubioxides' `Generator::biome_at` vs cubiomes'
//! end-to-end `setupGenerator + applySeed + getBiomeAt`. Reads the
//! binary fixture (kind = 46) and compares the chosen biome id for
//! 1024 random `(mc, flags, dim, seed, scale, x, y, z)` combinations
//! spanning every supported MC × dimension × scale path.

#![allow(clippy::missing_panics_doc)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::generator::Generator;
use cubioxides::mc_version::{Dimension, MCVersion};

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 46;

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
struct GeneratorBiomeRecord {
    mc: u32,
    flags: u32,
    dim: i32,
    scale: i32,
    seed: u64,
    x: i32,
    y: i32,
    z: i32,
    biome_id: i32,
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("fixtures")
        .join("layers")
        .join("generator_biome.bin")
}

fn mc_from_ord(ord: u32) -> MCVersion {
    match ord {
        1 => MCVersion::B1_7,
        3 => MCVersion::V1_0,
        10 => MCVersion::V1_7,
        15 => MCVersion::V1_12,
        19 => MCVersion::V1_16_1,
        22 => MCVersion::V1_18,
        25 => MCVersion::V1_20,
        28 => MCVersion::V1_21,
        other => panic!("unsupported MC ordinal: {other}"),
    }
}

fn dim_from_ord(ord: i32) -> Dimension {
    match ord {
        -1 => Dimension::Nether,
        0 => Dimension::Overworld,
        1 => Dimension::End,
        other => panic!("unsupported dim ord: {other}"),
    }
}

#[test]
fn generator_biome_at_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (header_bytes, body_bytes) = bytes.split_at(std::mem::size_of::<Header>());
    let header: &Header = bytemuck::from_bytes(header_bytes);
    assert_eq!(header.magic, MAGIC);
    assert_eq!(header.format_version, FORMAT_VERSION);
    assert_eq!(header.kind, KIND);
    let records: &[GeneratorBiomeRecord] = bytemuck::cast_slice(body_bytes);
    assert_eq!(records.len() as u64, header.record_count);

    for (i, rec) in records.iter().enumerate() {
        let mc = mc_from_ord(rec.mc);
        let dim = dim_from_ord(rec.dim);
        let mut g = Generator::new(mc, rec.flags);
        g.apply_seed(dim, rec.seed);
        let got = g.biome_at(rec.scale, rec.x, rec.y, rec.z);
        assert_eq!(
            got.id(),
            rec.biome_id,
            "biome mismatch at {i} (mc={:?}, flags={:#x}, dim={:?}, scale={}, seed={:#x}, x={}, y={}, z={}): got {}, want {}",
            mc,
            rec.flags,
            dim,
            rec.scale,
            rec.seed,
            rec.x,
            rec.y,
            rec.z,
            got.id(),
            rec.biome_id
        );
    }
}
