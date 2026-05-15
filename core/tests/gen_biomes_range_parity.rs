//! Parity test: cubioxides' `Generator::gen_biomes` Range API vs
//! cubiomes' end-to-end `setupGenerator + applySeed + genBiomes`.
//! Reads the binary fixture produced by `fixtures-gen layers`
//! (kind = 47, 256 records) and compares the XOR-folded digest of
//! the output cuboid for every supported `(mc, dim, scale, sx, sy,
//! sz)` combination.

#![allow(clippy::missing_panics_doc)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::biome::Biome;
use cubioxides::generator::{Generator, Range};
use cubioxides::mc_version::{Dimension, MCVersion};

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 47;

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
struct GenBiomesRangeRecord {
    mc: u32,
    flags: u32,
    dim: i32,
    scale: i32,
    seed: u64,
    x: i32,
    z: i32,
    sx: u32,
    sz: u32,
    y: i32,
    sy: u32,
    digest: u32,
    pad: u32,
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("fixtures")
        .join("layers")
        .join("gen_biomes_range.bin")
}

fn mc_from_ord(ord: u32) -> MCVersion {
    match ord {
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

fn hash32(mut x: u32) -> u32 {
    x ^= x >> 15;
    x = x.wrapping_mul(0xd168_aaad);
    x ^= x >> 15;
    x = x.wrapping_mul(0xaf72_3597);
    x ^= x >> 15;
    x
}

#[test]
fn gen_biomes_range_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (header_bytes, body_bytes) = bytes.split_at(std::mem::size_of::<Header>());
    let header: &Header = bytemuck::from_bytes(header_bytes);
    assert_eq!(header.magic, MAGIC);
    assert_eq!(header.format_version, FORMAT_VERSION);
    assert_eq!(header.kind, KIND);
    let records: &[GenBiomesRangeRecord] = bytemuck::cast_slice(body_bytes);
    assert_eq!(records.len() as u64, header.record_count);

    for (i, rec) in records.iter().enumerate() {
        let mc = mc_from_ord(rec.mc);
        let dim = dim_from_ord(rec.dim);
        let mut g = Generator::new(mc, rec.flags);
        g.apply_seed(dim, rec.seed);

        let range = Range {
            scale: rec.scale,
            x: rec.x,
            z: rec.z,
            sx: rec.sx,
            sz: rec.sz,
            y: rec.y,
            sy: rec.sy,
        };
        let cells = (rec.sx * rec.sy * rec.sz) as usize;
        let mut cache = vec![Biome::NONE; cells];
        g.gen_biomes(&mut cache, range);

        let mut digest: u32 = 0;
        for b in &cache {
            digest ^= hash32(b.id() as u32);
        }
        assert_eq!(
            digest, rec.digest,
            "gen_biomes digest mismatch at {i} (mc={:?}, dim={:?}, scale={}, seed={:#x}, x={}, z={}, sx={}, sz={}, y={}, sy={})",
            mc, dim, rec.scale, rec.seed, rec.x, rec.z, rec.sx, rec.sz, rec.y, rec.sy
        );
    }
}
