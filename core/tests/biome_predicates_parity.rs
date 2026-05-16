//! Parity test for `Biome::biome_exists`, `Biome::is_overworld_id`,
//! and `finder::is_stronghold_biome` against cubiomes. Cycles
//! through (mc, id) for id ∈ 0..256 across 11 MC versions
//! (Beta 1.7 through 1.21 WD).

#![allow(clippy::missing_panics_doc)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::biome::Biome;
use cubioxides::finder::is_stronghold_biome;
use cubioxides::mc_version::MCVersion;

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 53;

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
struct BiomePredicateRecord {
    mc: i32,
    id: i32,
    exists: i32,
    is_overworld: i32,
    is_stronghold: i32,
    pad: i32,
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("biome_predicates.bin")
}

fn mc_from_ord(ord: i32) -> MCVersion {
    match ord {
        1 => MCVersion::B1_7,
        2 => MCVersion::B1_8,
        3 => MCVersion::V1_0,
        4 => MCVersion::V1_1,
        9 => MCVersion::V1_6,
        10 => MCVersion::V1_7,
        15 => MCVersion::V1_12,
        16 => MCVersion::V1_13,
        22 => MCVersion::V1_18,
        25 => MCVersion::V1_20,
        28 => MCVersion::V1_21,
        other => panic!("unsupported MC ordinal: {other}"),
    }
}

#[test]
fn biome_predicates_match_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, KIND);
    let recs: &[BiomePredicateRecord] = bytemuck::cast_slice(body);
    assert_eq!(recs.len() as u64, h.record_count);

    for (i, r) in recs.iter().enumerate() {
        let mc = mc_from_ord(r.mc);
        let exists = Biome::biome_exists(mc, r.id);
        let is_ow = Biome::is_overworld_id(mc, r.id);
        let is_sh = is_stronghold_biome(mc, r.id);
        assert_eq!(
            exists,
            r.exists != 0,
            "biome_exists mismatch at {i} (mc={:?}, id={})",
            mc,
            r.id
        );
        assert_eq!(
            is_ow,
            r.is_overworld != 0,
            "is_overworld_id mismatch at {i} (mc={:?}, id={})",
            mc,
            r.id
        );
        assert_eq!(
            is_sh,
            r.is_stronghold != 0,
            "is_stronghold_biome mismatch at {i} (mc={:?}, id={})",
            mc,
            r.id
        );
    }
}
