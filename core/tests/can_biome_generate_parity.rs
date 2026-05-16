//! `can_biome_generate` parity vs cubiomes' `canBiomeGenerate`.
//! Cross-checks every (layer, mc, flags, biome_id) tuple cubiomes
//! supports.

#![allow(clippy::missing_panics_doc, clippy::doc_markdown)]

use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

use bytemuck::{Pod, Zeroable, cast_slice};
use cubioxides::finder::can_biome_generate::can_biome_generate;
use cubioxides::layer::LayerId;
use cubioxides::mc_version::MCVersion;

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct Header {
    magic: [u8; 4],
    format_version: u16,
    kind: u16,
    record_count: u64,
    reserved: [u64; 2],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct CanBiomeGenerateRecord {
    layer_id: i32,
    mc: i32,
    flags: u32,
    biome_id: i32,
    result: i32,
    padding: i32,
}

const MAGIC: [u8; 4] = *b"CUBX";
const FORMAT_VERSION: u16 = 1;
const KIND: u16 = 87;

fn fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .join("fixtures/layers")
        .join("can_biome_generate.bin")
}

fn layer_from_ord(o: i32) -> LayerId {
    match o {
        21 => LayerId::Biome256,
        22 => LayerId::Bamboo256,
        24 => LayerId::Zoom64,
        25 => LayerId::BiomeEdge64,
        29 => LayerId::Hills64,
        30 => LayerId::Sunflower64,
        33 => LayerId::Zoom16,
        34 => LayerId::Shore16,
        35 => LayerId::SwampRiver16,
        47 => LayerId::RiverMix4,
        48 => LayerId::OceanTemp256,
        55 => LayerId::OceanMix4,
        56 => LayerId::Voronoi1,
        _ => panic!("unsupported layer ord {o}"),
    }
}

fn mc_from_ord(o: i32) -> MCVersion {
    match o {
        3 => MCVersion::V1_0,
        10 => MCVersion::V1_7,
        12 => MCVersion::V1_9,
        17 => MCVersion::V1_14,
        19 => MCVersion::V1_16_1,
        22 => MCVersion::V1_18,
        28 => MCVersion::V1_21,
        _ => panic!("unsupported mc ord {o}"),
    }
}

#[test]
fn can_biome_generate_matches_cubiomes() {
    let path = fixture_path();
    let mut file = File::open(&path).unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read");
    let (h_bytes, body) = bytes.split_at(std::mem::size_of::<Header>());
    let h: &Header = bytemuck::from_bytes(h_bytes);
    assert_eq!(h.magic, MAGIC);
    assert_eq!(h.format_version, FORMAT_VERSION);
    assert_eq!(h.kind, KIND);
    let recs: &[CanBiomeGenerateRecord] = cast_slice(body);
    assert_eq!(recs.len() as u64, h.record_count);

    let mut diffs: u32 = 0;
    for (i, r) in recs.iter().enumerate() {
        let layer = layer_from_ord(r.layer_id);
        let mc = mc_from_ord(r.mc);
        let got = i32::from(can_biome_generate(layer, mc, r.flags, r.biome_id));
        if got != r.result {
            if diffs < 10 {
                eprintln!(
                    "record {i} (layer={:?}, mc={:?}, flags={:#x}, biome={}): rust {} vs cubiomes {}",
                    layer, mc, r.flags, r.biome_id, got, r.result
                );
            }
            diffs += 1;
        }
    }
    assert_eq!(diffs, 0, "can_biome_generate mismatched {diffs} records");
}
