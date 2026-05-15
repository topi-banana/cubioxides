//! Parity test: cubioxides' `EndNoise` vs cubiomes' `setEndSeed` /
//! `mapEndBiome` / `mapEnd`. Reads the binary fixture produced by
//! `fixtures-gen noise` (kind = 42) and compares grid digests at
//! various MC versions, seeds, and coordinates.

#![allow(clippy::missing_panics_doc)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::biomenoise::EndNoise;
use cubioxides::mc_version::MCVersion;

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 42;

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
struct EndRecord {
    mc: u32,
    w: u32,
    h: u32,
    _pad0: u32,
    seed: u64,
    x: i32,
    z: i32,
    biome_digest: u32,
    end_digest: u32,
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("fixtures")
        .join("noise")
        .join("end.bin")
}

fn mc_from_ord(ord: u32) -> MCVersion {
    match ord {
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
fn end_noise_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (header_bytes, body_bytes) = bytes.split_at(std::mem::size_of::<Header>());
    let header: &Header = bytemuck::from_bytes(header_bytes);
    assert_eq!(header.magic, MAGIC);
    assert_eq!(header.format_version, FORMAT_VERSION);
    assert_eq!(header.kind, KIND);
    let records: &[EndRecord] = bytemuck::cast_slice(body_bytes);
    assert_eq!(records.len() as u64, header.record_count);

    for (i, rec) in records.iter().enumerate() {
        let mc = mc_from_ord(rec.mc);
        let en = EndNoise::set_seed(mc, rec.seed);
        let w = rec.w as usize;
        let h = rec.h as usize;

        let mut out_biome = vec![0i32; w * h];
        en.map_end_biome(&mut out_biome, rec.x, rec.z, w, h);
        let mut d_biome: u32 = 0;
        for v in &out_biome {
            d_biome ^= hash32(*v as u32);
        }
        assert_eq!(
            d_biome, rec.biome_digest,
            "map_end_biome digest mismatch at {i} (mc={:?}, seed={:#x}, x={}, z={}, w={}, h={})",
            mc, rec.seed, rec.x, rec.z, rec.w, rec.h
        );

        let mut out_end = vec![0i32; w * h];
        en.map_end(&mut out_end, rec.x, rec.z, w, h);
        let mut d_end: u32 = 0;
        for v in &out_end {
            d_end ^= hash32(*v as u32);
        }
        assert_eq!(
            d_end, rec.end_digest,
            "map_end digest mismatch at {i} (mc={:?}, seed={:#x}, x={}, z={}, w={}, h={})",
            mc, rec.seed, rec.x, rec.z, rec.w, rec.h
        );
    }
}
