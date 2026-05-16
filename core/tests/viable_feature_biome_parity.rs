//! Parity test for `is_viable_feature_biome` vs cubiomes'
//! `isViableFeatureBiome`. Exercises every `(mc, structure_type,
//! biome_id)` triple in the supported matrix.

#![allow(clippy::missing_panics_doc)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable};
use cubioxides::finder::{StructureType, is_viable_feature_biome};
use cubioxides::mc_version::MCVersion;

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 66;

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
struct ViableFeatureBiomeRecord {
    mc: i32,
    structure_type: i32,
    biome_id: i32,
    viable: i32,
}

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("viable_feature_biome.bin")
}

fn mc_from_ord(ord: i32) -> MCVersion {
    match ord {
        3 => MCVersion::V1_0,
        5 => MCVersion::V1_2,
        8 => MCVersion::V1_5,
        11 => MCVersion::V1_8,
        14 => MCVersion::V1_11,
        16 => MCVersion::V1_13,
        17 => MCVersion::V1_14,
        19 => MCVersion::V1_16_1,
        20 => MCVersion::V1_16,
        22 => MCVersion::V1_18,
        25 => MCVersion::V1_20,
        28 => MCVersion::V1_21,
        other => panic!("unsupported MC ordinal: {other}"),
    }
}

#[test]
fn viable_feature_biome_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h_bytes, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h_bytes);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, KIND);
    let recs: &[ViableFeatureBiomeRecord] = bytemuck::cast_slice(body);
    assert_eq!(recs.len() as u64, h.record_count);

    for r in recs {
        let mc = mc_from_ord(r.mc);
        let sty = StructureType::from_ord(r.structure_type)
            .unwrap_or_else(|| panic!("unknown structure type ord {}", r.structure_type));
        let got = is_viable_feature_biome(mc, sty, r.biome_id);
        let want = r.viable != 0;
        assert!(
            got == want,
            "viable_feature_biome mismatch (mc={mc:?}, struct={sty:?}, biome={}): got {}, want {}",
            r.biome_id,
            got,
            want,
        );
    }
}
