//! Parity test: cubioxides' `climate_to_biome` vs cubiomes'
//! `climateToBiome`. Reads the binary fixture produced by
//! `fixtures-gen noise` (kind = 43) and compares the returned biome
//! id for 2048 random climate-tuple / MC pairs spanning the five
//! supported decision trees (1.18, 1.19.2, 1.19.4, 1.20.6, 1.21 WD).

#![allow(clippy::missing_panics_doc)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::biomenoise::climate_to_biome;
use cubioxides::mc_version::MCVersion;

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 43;

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
struct ClimateRecord {
    mc: u32,
    biome_id: i32,
    np: [u64; 6],
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root")
        .join("fixtures")
        .join("noise")
        .join("climate.bin")
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
fn climate_to_biome_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (header_bytes, body_bytes) = bytes.split_at(std::mem::size_of::<Header>());
    let header: &Header = bytemuck::from_bytes(header_bytes);
    assert_eq!(header.magic, MAGIC);
    assert_eq!(header.format_version, FORMAT_VERSION);
    assert_eq!(header.kind, KIND);
    let records: &[ClimateRecord] = bytemuck::cast_slice(body_bytes);
    assert_eq!(records.len() as u64, header.record_count);
    assert!(!records.is_empty());

    for (i, rec) in records.iter().enumerate() {
        let mc = mc_from_ord(rec.mc);
        let got = climate_to_biome(mc, &rec.np, None);
        assert_eq!(
            got, rec.biome_id,
            "climate_to_biome mismatch at {i} (mc={:?}, np={:?}): got {got}, want {}",
            mc, rec.np, rec.biome_id
        );
    }
}
