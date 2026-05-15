//! Parity test: cubioxides' `BiomeNoise::sample` vs cubiomes'
//! `initBiomeNoise` + `setBiomeSeed` + `sampleBiomeNoise`. Reads the
//! binary fixture produced by `fixtures-gen noise` (kind = 44) and
//! compares the chosen biome id plus the underlying 6-axis `np`
//! tuple for 512 random `(mc, seed, large, x, y, z)` combinations
//! over the five 1.18+ MC versions.

#![allow(clippy::missing_panics_doc)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::biomenoise::BiomeNoise;
use cubioxides::mc_version::MCVersion;

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 44;

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
struct BiomeNoiseRecord {
    mc: u32,
    large: u32,
    seed: u64,
    x: i32,
    y: i32,
    z: i32,
    biome_id: i32,
    np: [i64; 6],
    pad: [u32; 2],
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("fixtures")
        .join("noise")
        .join("biome_noise.bin")
}

fn mc_from_ord(ord: u32) -> MCVersion {
    match ord {
        22 => MCVersion::V1_18,
        23 => MCVersion::V1_19_2,
        24 => MCVersion::V1_19,
        25 => MCVersion::V1_20,
        28 => MCVersion::V1_21,
        other => panic!("unsupported MC ordinal: {other}"),
    }
}

#[test]
fn biome_noise_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (header_bytes, body_bytes) = bytes.split_at(std::mem::size_of::<Header>());
    let header: &Header = bytemuck::from_bytes(header_bytes);
    assert_eq!(header.magic, MAGIC);
    assert_eq!(header.format_version, FORMAT_VERSION);
    assert_eq!(header.kind, KIND);
    let records: &[BiomeNoiseRecord] = bytemuck::cast_slice(body_bytes);
    assert_eq!(records.len() as u64, header.record_count);
    assert!(!records.is_empty());

    for (i, rec) in records.iter().enumerate() {
        let mc = mc_from_ord(rec.mc);
        let bn = BiomeNoise::new(mc, rec.seed, rec.large != 0);
        let (got_id, got_np) = bn.sample(rec.x, rec.y, rec.z, 0);
        assert_eq!(
            got_np, rec.np,
            "np mismatch at record {i} (mc={:?}, seed={:#x}, large={}, x={}, y={}, z={}): \
             got {got_np:?}, want {:?}",
            mc, rec.seed, rec.large, rec.x, rec.y, rec.z, rec.np
        );
        assert_eq!(
            got_id, rec.biome_id,
            "biome id mismatch at record {i} (mc={:?}, seed={:#x}, large={}, x={}, y={}, z={}): \
             got {got_id}, want {}",
            mc, rec.seed, rec.large, rec.x, rec.y, rec.z, rec.biome_id
        );
    }
}
